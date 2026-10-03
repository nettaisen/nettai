--!strict
-- BlizzardMan, the navi of BN5's BlizMan chips (BN5's actor object #0x14, 0x080BC100; BN5's own, built on BN6's navi
-- template), brought by their navi chip controller. He appears (and goes at once off a panel that isn't solid),
-- stands 20 ticks, and winds up (animation 9): 6 ticks in, with its sound, he rolls a snowball (BlzrdBal's,
-- @common/blzrdbal, as BN5 has it: his sprite's animation 18, 19 grown, 100 more damage a swallow, silent) from 20
-- pixels ahead of him; 70 ticks on he leaves, and the controller moves on.

local navi_chips = require("@common/navi-chips/navi")
local balls = require("@common/blzrdbal/ball")
local rollers = require("@common/blzrdbal/roller")
local collision = require("../../rules/collision")

export type State = {
    -- The controller waiting for him (the flag it gave him, +0x54).
    controller: Object?,
    -- The chip's level (its Param1: 0, 4 for DS), which picks his palette.
    level: number,
}

local blizman = {}

local SPRITE = asset.sprite("bn5:blizzardman")
local SOUND_APPEAR = asset.sound("bn5:appear")
local SOUND_ROLL = asset.sound("bn5:blizzard-windup")
-- By the level (0x080BC178): DS's 3 (SP's level is 0).
local PALETTES: { [number]: number } = { [0] = 0, 0, 0, 0, 3 }
local SOLID = 0x10
-- When the ball goes and his arm comes down (ticks left of his windup).
local THROW, DOWN = 0x40, 0x22
-- His snowball's roller (BN5's attack object #0xB7, 0x080DD234): rows 0x48 and 0x49.
local ROLLER = rollers.make_kind {
    id = "bn5:blizman/roller",
    self_type = collision.hitbox,
    target = define.collision { id = "bn5:blizman/roller-reach", side0 = 0x3F800000, side1 = 0x3F800000, row_offset = 0x248 },
}
-- His snowball (BN5's attack object #0xB2, 0x080DC880).
local BALL = balls.make_kind {
    id = "bn5:blizman/ball",
    sprite = SPRITE,
    anim = 0x12,
    grown = { 0x13, 0x13 },
    roller = ROLLER,
    self_type = collision.attack,
    target = collision.hits_navis,
    growth = 100,
    grow_sound = nil,
}
blizman.ball, blizman.roller = BALL, ROLLER

-- Enter the phase with a timer (a word store clears Timer2 too), or count down; true once it has run out.
local function countdown(me: Object, ticks: number, word: boolean): boolean
    if me.phase_init == 0 then
        me.phase_init = 4
        me.timer = ticks
        if word then
            me.timer2 = 0
        end
        return false
    end
    local left = me.timer - 1
    me.timer = int.u16(left)
    return left <= 0
end

-- Action 0 (0x080BC1A4): he appears; 6 ticks; then he stays, or goes off a panel that isn't solid.
local function appear(me: Object)
    if me.phase_init == 0 then
        me.anim = 3
        battle.play_sound(SOUND_APPEAR)
        me.visible = true
    end
    if countdown(me, 6, true) then
        me:set_action(if field.check(me.panel_x, me.panel_y, SOLID, 0) then 4 else 0xC)
    end
end

-- Action 4 (0x080BC204): 20 ticks standing.
local function stand(me: Object)
    if me.phase_init == 0 then
        me.anim = 0
    end
    if countdown(me, 0x14, true) then
        me:set_action(8)
    end
end

-- Action 8 (0x080BC22C): his windup (animation 9), 71 ticks: at 64 left the snowball, with its sound; at 34 his
-- arm down.
local function roll(me: Object)
    if me.phase_init == 0 then
        me.phase_init = 4
        me.anim = 9
        me.timer = 0x46
        return
    end
    local left = me.timer - 1
    me.timer = int.u16(left)
    if left == THROW then
        battle.play_sound(SOUND_ROLL)
        local p = me.pos
        balls.spawn(BALL, me, p.x + me.facing * 0x14_0000, p.y, me.element, bit32.bor(me.damage, bit32.lshift(me.stamina, 16)))
    elseif left == DOWN then
        me.anim = 0
    elseif left < 0 then
        me:set_action(0xC)
    end
end

-- Action 0xC (0x080BC286): 12 ticks; then the controller moves on, and he goes.
local function leave(me: Object)
    if me.phase_init == 0 then
        me.anim = 4
    end
    if countdown(me, 0xC, true) then
        navi_chips.leave(me, (me.state :: State).controller)
    end
end

local ACTIONS = { [0] = appear, [4] = stand, [8] = roll, [0xC] = leave }

-- 0x080BC124: on his panel, on the ground; his sprite (animation 0, his shadow), his palette by the level, facing
-- his side's way.
local function init(me: Object)
    navi_chips.stand(me, SPRITE, "ground", true)
    me.sprite.palette = PALETTES[(me.state :: State).level] or 0
    me:set_lifecycle("update")
end

blizman.kind = define.kind {
    id = "bn5:blizman/navi",
    pool = "actor",
    state = { controller = "object", level = "u8" },
    -- 0x080BC100.
    update = function(me: Object)
        local l = me.lifecycle
        if l == "init" then
            init(me)
        elseif l == "update" then
            local action = ACTIONS[me.action]
            if not action then
                error(string.format("BlizzardMan action %#x reads past his table (0x080BC194)", me.action))
            end
            action(me)
        else
            me:free()
        end
        me:update_sprite_while_dimmed()
    end,
}

-- 0x080BC2B6: BlizzardMan for `user`, reporting to `controller` (the summon table's entry 14), of a chip of `level`.
function blizman.summon_at(level: number): (user: Object, controller: Object, spec: NaviChipSpec) -> Object?
    return function(user: Object, controller: Object, spec: NaviChipSpec): Object?
        local o = navi_chips.spawn(blizman.kind, user, spec)
        if not o then
            return nil
        end
        local s = o.state :: State
        s.controller = controller
        s.level = level
        return o
    end
end

-- BlizMan's and BlizManSP's (their Param1s 0), BlizManDS's (4).
blizman.summon = blizman.summon_at(0)
blizman.summon_sp = blizman.summon_at(0)
blizman.summon_dark = blizman.summon_at(4)

return blizman
