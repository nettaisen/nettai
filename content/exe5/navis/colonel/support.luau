--!strict
-- Colonel's party support (EXE5 DS: 0x02003C6C, his actor the support actor's variant 2, 08-07;
-- docs/design/exe5ds-map.md §4.7): once a turn, on the fight's second tick outside the dimming, he comes onto a panel of
-- his side's area up to five columns before the enemy farthest from the navi on the field, in its row, raises the tank
-- cannon and fires a shell ahead (the TankCans' shell, the DS's own row of it).

local attachment = require("../../objects/attachment")
local colonel = require("../colonel").colonel
local party = require("../../rules/party")
local party_support = require("../../lib/party_support")
local shell = require("@exelib/tankcan/shell")
local supports = require("../../rules/party/supports")
local tankcan = require("../../chips/tankcan/tankcan")

-- His damage by the side's level (0x020CC258), its hit flags 0x94.
local DAMAGE: { number } = { 30, 60, 80, 80, 100, 100, 120, 170, 180, 180 }
local FLAGS = 0x94
-- Where he may stand (0x020CC27C): 5, else 4, 3, 2, 1 columns in front of the enemy, in its row; by his side
-- (0x020CC26C): solid and his side's own, nothing on it or reserved.
local OFFSETS: { { { number } } } = { { { 5, 0, 0 } }, { { 4, 0, 0 } }, { { 3, 0, 0 } }, { { 2, 0, 0 } }, { { 1, 0, 0 } } }
local REQUIRE = { [0] = 0x10, [1] = 0x30 }
local FORBID = { [0] = 0x0F8800A0, [1] = 0x0F880080 }

-- His cannon: raised (animation 0xA, the tank cannon: the attachment table's row 0x25) for 8 ticks, held 30; fired
-- (animation 9, the cannon's animation 1): on its sixth tick a shake and the shell from the panel ahead, 16 pixels up,
-- its hit modifier the DS's shell row 2's (0x48), not cracking; 18 ticks; then 10 more before the cannon goes.
local ANIM_RAISE, ANIM_FIRE = 0xA, 9
local RAISE_TICKS, HOLD_TICKS, FIRE_AT, FIRE_ENDS, AFTER_TICKS = 8, 0x1E, 5, 0x12, 0xA
local SHELL: ShellSpec = { hit_mod = 0x48, cracks = false, shift = 0 }
local SHELL_Z = 0x10_0000
local RAISE, HOLD, FIRE, AFTER = 0x0, 0x4, 0x8, 0xC

type State = { member: number, held: Object? }

-- 0x021FE9AD: the cannon raised.
local function raise(me: Object)
    if me.phase_init == 0 then
        me.anim = ANIM_RAISE
        me.timer = RAISE_TICKS
        attachment.attach(me, tankcan.cannon, "held", { anim = 0, while_dimmed = true })
        me.phase_init = 4
        return
    end
    if party_support.countdown(me, RAISE_TICKS) then
        me.phase, me.phase_init = HOLD, 0
    end
end

-- 0x021FE9E1: held.
local function hold(me: Object)
    if party_support.countdown(me, HOLD_TICKS) then
        me.phase, me.phase_init = FIRE, 0
    end
end

-- 0x021FEA01: fired.
local function fire(me: Object)
    if me.phase_init == 0 then
        me.anim = ANIM_FIRE
        me.timer = 0
        local cannon = (me.state :: State).held
        if cannon then
            cannon.anim, cannon.anim_loaded = 1, 0xFF
        end
        me.phase_init = 4
        return
    end
    if me.timer == FIRE_AT then
        battle.shake_camera(1, 0x14)
        shell.fire(tankcan.kind, me, me.panel_x + me.facing, me.panel_y, SHELL_Z, bit32.bor(me.damage, bit32.lshift(me.stamina, 16)), SHELL)
    end
    me.timer += 1
    if me.timer >= FIRE_ENDS then
        me.phase, me.phase_init = AFTER, 0
    end
end

-- 0x021FEA55: after; the cannon ends.
local function after(me: Object)
    if party_support.countdown(me, AFTER_TICKS) then
        local cannon = (me.state :: State).held
        if cannon then
            cannon:set_lifecycle("destroy")
        end
        me.anim = 0
        party_support.go(me, party_support.WARP)
    end
end

local support = {}

support.actor = party_support.make {
    id = "colonel/support",
    navi = colonel,
    appear_ticks = 4,
    state = { held = "object" },
    actions = {
        [party_support.ATTACK] = function(me: Object)
            local p = me.phase
            if p == RAISE then
                raise(me)
            elseif p == HOLD then
                hold(me)
            elseif p == FIRE then
                fire(me)
            elseif p == AFTER then
                after(me)
            else
                error(string.format("Colonel's support: phase %#x reads past its table (0x0222712C)", p))
            end
        end,
    },
}

-- 0x02003C6C (each fight tick): once a turn (the party block's +6 and its counter +7, from 0x02003B98); the light test;
-- on the second tick outside the dimming, Colonel a member who may support (spent here, whatever comes next), the
-- side's navi not Colonel, no member busy, a panel by an enemy.
function support.tick(side: number)
    local s = party.supports(side)
    if s.colonel_used or not party.light(side) or battle.dimmed() then
        return
    end
    s.colonel_count = bit32.band(s.colonel_count + 1, 0xFF)
    if s.colonel_count ~= 2 then
        return
    end
    local m = party.member_of(side, colonel)
    if m == nil or not party.may_support(side, colonel) then
        return
    end
    s.colonel_used = true
    local navi = battle.player(side)
    if navi == nil or battle.navi(side).navi == colonel or party.any_busy(side) then
        return
    end
    local x, y, flip = supports.target(navi, OFFSETS, REQUIRE[side], FORBID[side], true)
    if x and y then
        party_support.send(support.actor, navi, x, y, flip, supports.damage(side, DAMAGE, FLAGS), m)
    end
end

return support
