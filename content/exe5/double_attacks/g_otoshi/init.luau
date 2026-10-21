--!strict
-- G-OTOSHI (the DS's chip 0x16B; Gama Otoshi), the Double Attack of ShadowMan and ToadMan (docs/design/exe5ds-map.md
-- §5, §5.2: its controller the DS's effect object 0xA2, its navis its T1 0x43, its frogs its attack object 0xA2). Both
-- sing (sound 0xD8); ToadMan drops twelve frogs (./frog), one every 11 ticks, each on a panel picked at random: with 10
-- chances in 16 one an enemy's body stands on, else any panel none does (the whole field: the user's side's too). A
-- frog hits once (Aqua) and turns its panel to sea, or turns it to sea as it lands on a solid one. The chip's record
-- and name are the DS's (written by tools/exe5/gen_content.py, bn6battle-verify, from the DS ROMs); its record's use is
-- the cannon's (action 0x14, subtype 0: the DS copied Cannon's record), which nothing starts: no folder or hand holds
-- it.
--
-- Its controller links the two navis (each its partner's, +0x60): ShadowMan sings until ToadMan is done.

local cannon_chips = require("../chips/cannon")
local double_attack = require("../lib/double_attack")
local frog = require("@self/frog")
local panels = require("@exelib/panels")
local shadowman = require("../navis/shadowman").shadowman
local toadman = require("../navis/toadman").toadman

type State = DoubleAttackNavi

local CHARGE_TICKS = 0x3C
-- The song: animation 0xC and its sound; ToadMan's twelve frogs, 11 ticks apart.
local ANIM_SING = 0xC
local SOUND_SONG = asset.sound("bblstar1")
local FROGS, FROG_TICKS = 0xC, 0xA
-- The frogs' panels (0x02135B68, by side and draw): an enemy body's (flag 0x04000000 for side 0, 0x08000000 for side
-- 1) when the draw's low four bits are under 10, else a panel without one.
local ENEMY_BODY = { [0] = 0x04000000, [1] = 0x08000000 }
local ON_ENEMY_UNDER = 0xA
local AFTER_TICKS = 0x1E

local function appear(me: Object)
    double_attack.appear(me, 4, 0x10)
end

-- 0x020A1B20: a frog on a panel at random (two draws; the second only when there is a panel).
local function drop_frog(me: Object)
    local body = ENEMY_BODY[me.alliance]
    local on_enemy = bit32.band(battle.rng_positive(), 0xF) < ON_ENEMY_UNDER
    local list = if on_enemy
        then panels.filtered(body, 0, me.alliance)
        else panels.filtered(0, body, me.alliance)
    if #list == 0 then
        return
    end
    local p = list[battle.rng_positive() % #list + 1]
    frog.spawn(me, panels.x(p), panels.y(p))
end

-- 0x020A1860 (ToadMan): the charge; then the panel ahead in row 1 his destination.
local function toad_charge(me: Object)
    if not double_attack.charge(me, CHARGE_TICKS) then
        return
    end
    me.future_panel_x, me.future_panel_y = me.panel_x + me.facing, 1
    me:set_action(8)
end

-- 0x020A18F4 (ToadMan): the song, and a frog every 11 ticks, twelve.
local function frogs(me: Object)
    if me.phase_init == 0 then
        me.timer2 = FROGS
        me.anim = ANIM_SING
        battle.play_sound(SOUND_SONG)
        me.timer = FROG_TICKS
        me.phase_init = 4
    end
    local t = me.timer - 1
    me.timer = t
    if t >= 0 then
        return
    end
    me.timer = FROG_TICKS
    drop_frog(me)
    local n = me.timer2 - 1
    me.timer2 = n
    if n <= 0 then
        me:set_action(0xC)
    end
end

-- 0x020A19E4 (ShadowMan): the charge; then a count of 30 (+0x22, which nothing reads).
local function shadow_charge(me: Object)
    if not double_attack.charge(me, CHARGE_TICKS) then
        return
    end
    me.timer2 = 0x1E
    me:set_action(8)
end

-- 0x020A1A6C (ShadowMan): the song, until ToadMan is done.
local function sing(me: Object)
    if me.phase_init == 0 then
        me.anim = ANIM_SING
        battle.play_sound(SOUND_SONG)
        me.phase_init = 4
    end
    if double_attack.partner(me).action == 0xC then
        me:set_action(0xC)
    end
end

-- 0x020A1934, 0x020A1A94: 30 ticks, then he leaves.
local function after(me: Object)
    if me.phase_init == 0 then
        me.timer = AFTER_TICKS
        me.phase_init = 4
    end
    if double_attack.ran_out(me) then
        me:set_action(0x10)
    end
end

local chip: Chip = {
    codes = { "A", "B", "C", "*" },
    element = "null",
    family = "special",
    class = "special",
    rarity = 0,
    mb = 5,
    damage = 40,
    hit_param = 138,
    lockout = 0,
    flags = {},
    action = cannon_chips.cannon.action,
    double_attack = double_attack.make {
        id = "g-otoshi",
        damage = { 30, 30, 30, 30, 30, 30, 40, 50, 60, 60 },
        roles = {
            -- 0x020A1808's table: ToadMan, (10, 10) from the user's spot.
            {
                navi = toadman,
                dx = 10,
                dy = 10,
                actions = {
                    [0x0] = appear,
                    [0x4] = toad_charge,
                    [0x8] = frogs,
                    [0xC] = after,
                    [0x10] = function(me: Object)
                        double_attack.leave(me, true)
                    end,
                },
            },
            -- 0x020A198C's: ShadowMan, (-10, -10).
            {
                navi = shadowman,
                dx = -10,
                dy = -10,
                actions = {
                    [0x0] = appear,
                    [0x4] = shadow_charge,
                    [0x8] = sing,
                    [0xC] = after,
                    [0x10] = function(me: Object)
                        double_attack.leave(me, false)
                    end,
                },
            },
        },
    },
}

return { ["g-otoshi"] = chip }
