--!strict
-- NapalmMan's party support (EXE5 DS: 0x02003A14, his actor the support actor's variant 1, 08-04;
-- docs/design/exe5ds-map.md §4.7): while he is benched, every 130 to 250 ticks outside the dimming he comes onto a panel
-- of his side's area up to five columns before the enemy nearest the navi on the field, in its row, and fires five
-- vulcan shots ahead; his timer starts again as he goes (or when he can't come).

local attachment = require("../../objects/attachment")
local bullet = require("../../objects/bullet")
local bullets = require("@exelib/bullet")
local napalmman = require("../napalmman").napalmman
local party = require("../../rules/party")
local party_support = require("../../lib/party_support")
local regions = require("@exelib/regions")
local supports = require("../../rules/party/supports")

-- His damage a shot by the side's level (0x020CC204), its hit flags 0x82.
local DAMAGE: { number } = { 4, 4, 4, 4, 6, 7, 10, 15, 20, 20 }
local FLAGS = 0x82
-- His timer (0x02003BB0: RNG2 bits 6–9 into 0x020CC248's offsets of 0x020CC240's ticks, weights 2/6/6/2).
local TIMER: { number } = { 130, 130, 170, 170, 170, 170, 170, 170, 210, 210, 210, 210, 210, 210, 250, 250 }
-- Where he may stand (0x020CC228): 5, else 4, 3, 2, 1 columns in front of the enemy, in its row; by his side
-- (0x020CC218): solid and his side's own (side 1's flag 0x20), nothing on it or reserved.
local OFFSETS: { { { number } } } = { { { 5, 0, 0 } }, { { 4, 0, 0 } }, { { 3, 0, 0 } }, { { 2, 0, 0 } }, { { 1, 0, 0 } } }
local REQUIRE = { [0] = 0x10, [1] = 0x30 }
local FORBID = { [0] = 0x0F8800A0, [1] = 0x0F880080 }

-- His shots (five), each: his firing animation 0xE, on its second tick the shot's sound, a bullet from the panel ahead
-- at a height drawn from RNG2 (8, 16, 24 or 32 pixels: 0x02227114) and his muzzle flash (the attachment table's row
-- 0xD); three ticks after, a wait of three, his flash let go.
local SHOTS = 5
local ANIM_FIRE = 0xE
local SHOT_SOUND = asset.sound("col-army-2")
local HEIGHTS = { 0x08_0000, 0x10_0000, 0x18_0000, 0x20_0000 }
local FLASH = attachment.look { sprite = asset.sprite("muzzle-flash"), palette = 0, attach_point = 0x10 }
-- The DS's bullet row 0x13 (0x02135BAC): the panel found and the one behind, no hit modifier, the vulcan hit in its
-- animation 1 (effect 0x58) at the bullet.
local SHOT = bullet.variant {
    region = regions.long,
    hit_mod = 0,
    effect = new.effect { sprite = asset.sprite("vulcan-hit"), anim = 1, palette = 0 },
    single = true,
}
local FIRE, WAIT = 0, 4

type State = { member: number, shots: number }

local support = {}

-- 0x02003BB0: his timer starts again.
local function reset(side: number)
    party.supports(side).napalmman_timer = supports.pick(TIMER, 6, true)
end

-- 0x021FE7F9: a shot.
local function fire(me: Object)
    if me.phase_init == 0 then
        me.anim = ANIM_FIRE
        me.phase_init = 4
        me.timer = 0
    end
    if me.timer == 1 then
        battle.play_sound(SHOT_SOUND)
        local z = HEIGHTS[bit32.band(battle.rng_positive(), 3) + 1]
        bullets.fire(bullet.kind, me, me.panel_x + me.facing, me.panel_y, z, SHOT, me.element, bit32.bor(me.damage, bit32.lshift(me.stamina, 16)))
        attachment.attach(me, FLASH, "related", { anim = 0, while_dimmed = true })
    end
    me.timer += 1
    if me.timer > 2 then
        me.phase, me.phase_init = WAIT, 0
    end
end

-- 0x021FE83D: the wait after a shot; the next, or the warp out.
local function wait(me: Object)
    if me.phase_init == 0 then
        me.timer = 2
        me.phase_init = 4
    end
    local t = me.timer - 1
    me.timer = t
    if t >= 0 then
        return
    end
    me.related1 = nil
    local s = me.state :: State
    s.shots -= 1
    if s.shots > 0 then
        me.phase, me.phase_init = FIRE, 0
    else
        party_support.go(me, party_support.WARP)
    end
end

support.actor = party_support.make {
    id = "napalmman/support",
    navi = napalmman,
    appear_ticks = 4,
    state = { shots = "u16" },
    actions = {
        [party_support.ATTACK] = function(me: Object)
            if me.phase == FIRE then
                fire(me)
            elseif me.phase == WAIT then
                wait(me)
            else
                error(string.format("NapalmMan's support: phase %#x reads past its table (0x0222710C)", me.phase))
            end
        end,
    },
    appeared = function(me: Object)
        (me.state :: State).shots = SHOTS
    end,
    gone = function(me: Object)
        reset(me.alliance)
    end,
}

-- 0x02003A14 (each fight tick): the light test, NapalmMan a benched member who may support; his timer set once a turn;
-- outside the dimming, his timer runs out: the side's navi not NapalmMan, no member busy, a panel by an enemy; else his
-- timer starts again.
function support.tick(side: number)
    if not party.light(side) then
        return
    end
    local m = party.member_of(side, napalmman)
    if m == nil or not party.may_support(side, napalmman) or not party.benched(side, napalmman) then
        return
    end
    local s = party.supports(side)
    if not s.napalmman_set then
        s.napalmman_set = true
        reset(side)
    end
    if battle.dimmed() then
        return
    end
    s.napalmman_timer = bit32.band(s.napalmman_timer - 1, 0xFFFF)
    if s.napalmman_timer ~= 0 then
        return
    end
    local navi = battle.player(side)
    if navi == nil or battle.navi(side).navi == napalmman then
        return
    end
    local x, y, flip
    if not party.any_busy(side) then
        x, y, flip = supports.target(navi, OFFSETS, REQUIRE[side], FORBID[side])
    end
    if x == nil or y == nil then
        reset(side)
        return
    end
    party_support.send(support.actor, navi, x, y, flip, supports.damage(side, DAMAGE, FLAGS), m)
end

return support
