--!strict
-- M-AIR (the DS's chip 0x162; Mag Air Raid), the Double Attack of GyroMan and MagnetMan (docs/design/exe5ds-map.md
-- §5, §5.2: its director the DS's effect object 0x99, its actors its T1 0x37, which the director tells of each other).
-- Neither checks its panel as it appears. GyroMan rises 40 pixels; MagnetMan, his magnet arm raised, sends a magnet
-- bomb up to him for each panel of the other side's from his column on (./toss: seen only), one every 5 ticks after
-- the first; then GyroMan flies ahead along the user's row, 2 pixels a tick, and over each column's center drops a
-- bomb on each of its panels of the other side's (./bomb: Elec, paralyzing), until he is off the screen. The chip's
-- record and name are the DS's (written by tools/exe5/gen_content.py, bn6battle-verify, from the DS ROMs); its
-- record's use is the cannon's (action 0x14, subtype 0: the DS copied Cannon's record), which nothing starts: no
-- folder or hand holds it.

local attachment = require("../objects/attachment")
local bomb = require("@self/bomb")
local cannon_chips = require("../chips/cannon")
local double_attack = require("../lib/double_attack")
local gyroman = require("../navis/gyroman").gyroman
local magnetman = require("../navis/magnetman").magnetman
local toss = require("@self/toss")

-- Its actors' state: the shared fields and GyroMan's next column's center (+0x74).
type State = DoubleAttackActor & { next_center: number }

local CHARGE_TICKS = 0x3C
local AFTER_TICKS = 0xA
local ANIM_IDLE = 0
-- GyroMan's rise: taking off (animation 0x12) 40 pixels up, in the air (0x14) from its 20th tick, over 41 ticks.
local ANIM_TAKE_OFF, ANIM_AIRBORNE = 0x12, 0x14
local RISEN, AIRBORNE_AT, RISE_TICKS = 0x28_0000, 0x13, 0x28
local SOUND_TAKE_OFF = asset.sound("gyro-change")
local SOUND_AIRBORNE = asset.sound("gyro-takeoff")
-- His flight: 2 pixels a tick; off the screen past 160 pixels either way.
local SPEED = 0x2_0000
local SCREEN = 0xA0
local SOUND_DROP = asset.sound("drop")
-- MagnetMan's arm (the DS's attachment row 0x3A, 10-3C), raised in his animation 0x10; a bomb
-- every 5 ticks.
local MAGNET_ARM = attachment.look { sprite = asset.sprite("magnet-arm"), palette = 0 }
local ANIM_SEND = 0x10
local SEND_TICKS = 5
-- GyroMan's and MagnetMan's actions: appear, charge, attack (by phase), after, leave.
local APPEAR, CHARGE, ATTACK, AFTER, LEAVE = 0x0, 0x4, 0x8, 0xC, 0x10

-- 0x020A55C0 (GyroMan), 0x020A57F8 (MagnetMan): it appears, 20 ticks, then charges, whatever its panel. GyroMan puts on
-- what he wears (his NameID's actor record is a player's: `sub_8011420` with Param3 1, his propeller).
local function appear(me: Object)
    double_attack.appear(me, CHARGE, nil, me.identity == gyroman.identity)
end

-- 0x020A5610: GyroMan charges; his destination the panel ahead in the top row (the DS also sets its +0x70, which
-- nothing reads).
local function gyro_charge(me: Object)
    if not double_attack.charge(me, CHARGE_TICKS) then
        return
    end
    me.future_panel_x, me.future_panel_y = me.panel_x + me.facing, 1
    me:set_action(ATTACK)
end

-- 0x020A5834: MagnetMan charges.
local function magnet_charge(me: Object)
    if double_attack.charge(me, CHARGE_TICKS) then
        me:set_action(ATTACK)
    end
end

-- 0x020A59E0: over a column: a bomb on each of its panels of the other side's, and the sound if any dropped.
local function drop(me: Object)
    if not field.valid(me.panel_x, me.panel_y) then
        return
    end
    local dropped = false
    for y = 1, 3 do
        local p = field.panel(me.panel_x, y) :: Panel
        if p.alliance ~= me.alliance then
            bomb.drop(me, me.panel_x, y, bit32.bor(me.damage, bit32.lshift(me.stamina, 16)))
            dropped = true
        end
    end
    if dropped then
        battle.play_sound(SOUND_DROP)
    end
end

-- The center of the column ahead of his, 16.16 (his next drop's).
local function next_center(me: Object): number
    local cx = field.center(me.panel_x + me.facing, me.panel_y)
    return cx
end

-- 0x020A56A4's phases: rise (0x020A56B4), wait for MagnetMan's bombs (0x020A5700), fly (0x020A5718).
local function gyro_attack(me: Object)
    local p = me.phase
    if p == 0x0 then
        if me.phase_init == 0 then
            me.anim = ANIM_TAKE_OFF
            battle.play_sound(SOUND_TAKE_OFF)
            me.timer = 0
            local pos = me.pos
            me.pos = Vec3.new(pos.x, pos.y, RISEN)
            me.phase_init = 4
        end
        if me.timer == AIRBORNE_AT then
            battle.play_sound(SOUND_AIRBORNE)
            me.anim = ANIM_AIRBORNE
        end
        local t = me.timer + 1
        me.timer = t
        if t > RISE_TICKS then
            me.phase, me.phase_init = 0x4, 0
        end
    elseif p == 0x4 then
        local m = double_attack.partner(me)
        if m.action == ATTACK and m.phase == 0x8 then
            me.phase, me.phase_init = 0x8, 0
        end
    elseif p == 0x8 then
        local s = me.state :: State
        if me.phase_init == 0 then
            local v = me.vel
            me.vel = Vec3.new(SPEED * me.facing, v.y, v.z)
            s.next_center = next_center(me)
            me.phase_init = 4
        end
        local pos = me.pos
        local old = pos.x
        me.pos = Vec3.new(old + me.vel.x, pos.y, pos.z)
        me:set_panel_from_coordinates()
        if field.passed(me.pos.x, old, s.next_center, me.alliance) then
            drop(me)
            s.next_center = next_center(me)
        end
        -- (The halfword of its X: signed pixels.)
        local x = int.asr(me.pos.x, 16)
        if bit32.band(x + SCREEN, 0xFFFFFFFF) > 2 * SCREEN then
            me.visible = false
            me:set_action(AFTER)
        end
    else
        error(string.format("M-AIR's GyroMan: phase %#x reads past his table (0x02139018)", p))
    end
end

-- 0x020A58B8's phases: wait for GyroMan up (0x020A58C8), send the bombs (0x020A58E0), wait for GyroMan done
-- (0x020A5938).
local function magnet_attack(me: Object)
    local s = me.state :: State
    local p = me.phase
    if p == 0x0 then
        local g = double_attack.partner(me)
        if g.action == ATTACK and g.phase == 0x4 then
            me.phase, me.phase_init = 0x4, 0
        end
    elseif p == 0x4 then
        if me.phase_init == 0 then
            me.anim = ANIM_SEND
            me.timer = 0
            -- 0x020A5A30: the panels of the other side's from his column on.
            local n = 0
            local x = me.panel_x
            while field.valid(x, 1) do
                for y = 1, 3 do
                    if (field.panel(x, y) :: Panel).alliance ~= me.alliance then
                        n += 1
                    end
                end
                x += me.facing
            end
            me.timer2 = n
            attachment.attach(me, MAGNET_ARM, "held", { anim = 0, while_dimmed = true })
            me.phase_init = 4
        end
        if double_attack.ran_out(me) then
            local n = me.timer2 - 1
            me.timer2 = n
            if n > 0 then
                me.timer = SEND_TICKS
                toss.send(me)
            else
                me.anim = ANIM_IDLE
                s.held = nil
                me.phase, me.phase_init = 0x8, 0
            end
        end
    elseif p == 0x8 then
        if double_attack.partner(me).action == AFTER then
            me:set_action(AFTER)
        end
    else
        error(string.format("M-AIR's MagnetMan: phase %#x reads past his table (0x02139038)", p))
    end
end

-- 0x020A5790 (GyroMan: his animation as it is), 0x020A594C (MagnetMan: idle, his slots let go): 10 ticks, then he
-- leaves.
local function after(me: Object)
    local s = me.state :: State
    if me.phase_init == 0 then
        if me.identity == magnetman.identity then
            me.anim = ANIM_IDLE
            s.held, s.held2 = nil, nil
        end
        me.timer = AFTER_TICKS
        me.phase_init = 4
    end
    if double_attack.ran_out(me) then
        me:set_action(LEAVE)
    end
end

local chip: Chip = {
    codes = { "A", "B", "C", "*" },
    element = "null",
    family = "special",
    class = "special",
    rarity = 0,
    mb = 0,
    damage = 40,
    hit_param = 168,
    lockout = 0,
    flags = {},
    action = cannon_chips.cannon.action,
    double_attack = double_attack.make {
        id = "m-air",
        damage = { 60, 60, 60, 70, 80, 90, 150, 180, 200, 200 },
        state = { next_center = "i32" },
        roles = {
            -- 0x020A55B0's table: GyroMan, (10, 10) from the user's spot.
            {
                navi = gyroman,
                dx = 10,
                dy = 10,
                actions = {
                    [APPEAR] = appear,
                    [CHARGE] = gyro_charge,
                    [ATTACK] = gyro_attack,
                    [AFTER] = after,
                    [LEAVE] = function(me: Object)
                        double_attack.leave(me, true)
                    end,
                },
            },
            -- 0x020A57E8's: MagnetMan, (-10, -10).
            {
                navi = magnetman,
                dx = -10,
                dy = -10,
                actions = {
                    [APPEAR] = appear,
                    [CHARGE] = magnet_charge,
                    [ATTACK] = magnet_attack,
                    [AFTER] = after,
                    [LEAVE] = function(me: Object)
                        double_attack.leave(me, false)
                    end,
                },
            },
        },
    },
}

return { ["m-air"] = chip }
