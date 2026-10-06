--!strict
-- TomahawkMan, the navi of EXE5's TomahawkMan chips (EXE5's actor object #0x13, 0x080BBED0; EXE5's own, built on EXE6's
-- navi template: EXE6's TomahawkMan, its chips/tmhkman, swings later and harder), brought by their navi chip
-- controller. He appears (and goes at once off a panel that isn't solid), stands 20 ticks, raises his tomahawk for
-- 30, and swings it: 10 ticks in it comes down on the panel ahead, a hit over that column and the next (no spark,
-- hit modifier 3) with the earth splashing there. 50 ticks on he goes, and the controller moves on.
--
-- Not reached: his other spawner (0x080BC0DA, which nothing calls), whose TomahawkMan keeps no controller waiting
-- (his Param3 1).

local navi_chips = require("@exelib/navi_chips/navi")
local collision = require("../../rules/collision")
local regions = require("@exelib/regions")

export type State = {
    -- The controller waiting for him (the flag it gave him, +0x54).
    controller: Object?,
    -- The chip's level (its Param1: 0, 3 for SP, 4 for DS), which picks his palette.
    level: number,
}

local tmhwkman = {}

local SPRITE = asset.sprite("tomahawkman")
local SOUND_APPEAR = asset.sound("appear")
local SOUND_SWING = asset.sound("tomahawk-man")
-- The earth his tomahawk splashes (the effect table's row 0x45).
local EARTH = define.effect { sprite = asset.sprite("big-slash"), anim = 1, palette = 0 }
-- By the level (0x0800DA8A: navi 10's count, 2, by 0x0800DAE8's row): SP's 4, DS's 6.
local PALETTES: { [number]: number } = { [0] = 0, 0, 0, 4, 6 }
local SOLID = 0x10
-- When his tomahawk lands (ticks left of his swing).
local STRIKE = 0x32

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

-- Action 0 (0x080BBF60): he appears; 4 ticks; then he stays, or goes off a panel that isn't solid.
local function appear(me: Object)
    if me.phase_init == 0 then
        me.anim = 3
        battle.play_sound(SOUND_APPEAR)
        me.visible = true
    end
    if countdown(me, 4, true) then
        me:set_action(if field.check(me.panel_x, me.panel_y, SOLID, 0) then 4 else 0x10)
    end
end

-- Action 4 (0x080BBFB0): 20 ticks standing.
local function stand(me: Object)
    if me.phase_init == 0 then
        me.anim = 0
    end
    if countdown(me, 0x14, true) then
        me:set_action(8)
    end
end

-- Action 8 (0x080BBFD8): 30 ticks raising the tomahawk.
local function raise(me: Object)
    if me.phase_init == 0 then
        me.anim = 0x10
    end
    if countdown(me, 0x1E, false) then
        me:set_action(0xC)
    end
end

-- The strike (0x080BC036): a hit over the panel ahead's column and the next, running while dimmed (`sub_80C53A6`:
-- region 0x11, no spark, target 0, self 7, hit modifier 3), and the earth splashing there, 16 pixels up.
local function strike(me: Object)
    local x, y = me.panel_x + me.facing, me.panel_y
    local box = battle.hitbox(me, {
        panel_x = x,
        panel_y = y,
        element = me.element,
        z = 0,
        region = regions.wide_long,
        hit_effect = nil,
        target = collision.nothing,
        self_type = collision.slash,
        damage = me.damage,
        stamina = me.stamina,
        hit_mod = 3,
    })
    if box then
        box.run_while_dimmed = true
    end
    local cx, cy = field.center(x, y)
    battle.effect(Vec3.new(cx, cy, 0x10_0000), EARTH, bit32.bxor(me.alliance, me.flip))
end

-- Action 0xC (0x080BC000): the swing (its sound), 61 ticks (to -1); the tomahawk lands at 50 left.
local function swing(me: Object)
    if me.phase_init == 0 then
        me.phase_init = 4
        battle.play_sound(SOUND_SWING)
        me.anim = 0x11
        me.timer = 0x3C
        return
    end
    local left = me.timer - 1
    me.timer = int.u16(left)
    if left == STRIKE then
        strike(me)
    elseif left < 0 then
        me:set_action(0x10)
    end
end

-- Action 0x10 (0x080BC080): 4 ticks; then the controller moves on, and he goes.
local function leave(me: Object)
    if me.phase_init == 0 then
        me.anim = 4
    end
    if countdown(me, 4, true) then
        navi_chips.leave(me, (me.state :: State).controller)
    end
end

local ACTIONS = { [0] = appear, [4] = stand, [8] = raise, [0xC] = swing, [0x10] = leave }

-- 0x080BBEF4: on his panel, on the ground, his palette by the level.
local function init(me: Object)
    navi_chips.stand(me, SPRITE, "ground", true)
    me.sprite.palette = PALETTES[(me.state :: State).level] or 0
    me:set_lifecycle("update")
end

tmhwkman.kind = define.kind {
    id = "tmhwkman/navi",
    pool = "actor",
    state = { controller = "object", level = "u8" },
    -- 0x080BBED0.
    update = function(me: Object)
        local l = me.lifecycle
        if l == "init" then
            init(me)
        elseif l == "update" then
            local action = ACTIONS[me.action]
            if not action then
                error(string.format("TomahawkMan action %#x reads past his table (0x080BBF4C)", me.action))
            end
            action(me)
        else
            me:free()
        end
        me:update_sprite_while_dimmed()
    end,
}

-- 0x080BC0B0: TomahawkMan for `user`, reporting to `controller` (the summon table's entry 12), of a chip of
-- `level`.
function tmhwkman.summon_at(level: number): (user: Object, controller: Object, spec: NaviChipSpec) -> Object?
    return function(user: Object, controller: Object, spec: NaviChipSpec): Object?
        local o = navi_chips.spawn(tmhwkman.kind, user, spec)
        if not o then
            return nil
        end
        local s = o.state :: State
        s.controller = controller
        s.level = level
        return o
    end
end

tmhwkman.summon = tmhwkman.summon_at(0)
tmhwkman.summon_sp = tmhwkman.summon_at(3)
tmhwkman.summon_dark = tmhwkman.summon_at(4)

return tmhwkman
