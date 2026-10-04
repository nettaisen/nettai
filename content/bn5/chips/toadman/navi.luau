--!strict
-- ToadMan, the navi of the ToadMan chips (BN5's actor object #0x17, 0x080BC748; BN5's own, built on BN6's navi
-- template), brought by their navi chip controller. He appears (and goes at once off a panel that isn't solid),
-- stands 10 ticks, readies 10, and plays: a chain of notes (./note) from the panel ahead toward the nearest enemy
-- combatant in his cone ahead (a row nearer it each step; none: straight ahead), paralyzing. 30 ticks of playing, 30
-- standing, and he leaves; the controller moves on.

local navi_chips = require("@exelib/navi-chips/navi")
local note = require("./note")

export type State = {
    -- The controller waiting for him (the flag it gave him, +0x54).
    controller: Object?,
    -- The chip's level (its Param1: 0, 4 for DS), which picks his palette.
    level: number,
}

local toadman = {}

local SPRITE = asset.sprite("bn5:toadman")
local SOUND_APPEAR = asset.sound("bn5:appear")
local SOUND_PLAY = asset.sound("bn5:toad-melody")
-- By the level (0x0800DA8A: navi 12's count, 1, by 0x0800DAE8's row): DS's 3 (SP's level is 0).
local PALETTES: { [number]: number } = { [0] = 0, 0, 0, 2, 3 }
-- His notes (0x080BC9D8): 10 ticks each, the next 6 ticks in, running while dimmed.
local CHAIN: { life: number, delay: number, dimmed: boolean } = { life = 0xA, delay = 6, dimmed = true }
local SOLID = 0x10

-- 0x080BC9AA: (x2, y2) lies in his cone from (x, y): not behind, and no more rows off than columns ahead.
local function in_cone(me: Object, x: number, y: number, x2: number, y2: number): boolean
    local ahead = (x2 - x) * me.facing
    if ahead < 0 then
        return false
    end
    return math.abs(y2 - y) <= ahead
end

-- 0x080BC92C: the enemy combatant in his cone from the panel ahead nearest it in columns (then the upper); none.
local function find_target(me: Object): Object?
    local x, y = me.panel_x + me.facing, me.panel_y
    local best: Object? = nil
    for _, o in battle.alive_actors(bit32.bxor(me.alliance, 1)) do
        if in_cone(me, x, y, o.panel_x, o.panel_y) then
            local b = best
            if not b then
                best = o
            else
                local d, db = math.abs(o.panel_x - x), math.abs(b.panel_x - x)
                if d < db or (d == db and o.panel_y < b.panel_y) then
                    best = o
                end
            end
        end
    end
    return best
end

-- Enter the phase with a timer (counting down from its first tick), or count down; true once it has run out.
local function countdown(me: Object, ticks: number, word: boolean?): boolean
    if me.phase_init == 0 then
        me.phase_init = 4
        me.timer = ticks
        if word then
            me.timer2 = 0
        end
    end
    local left = me.timer - 1
    me.timer = int.u16(left)
    return left <= 0
end

-- Action 0 (0x080BC7D4): he appears; 4 ticks.
local function appear(me: Object)
    if me.phase_init == 0 then
        battle.play_sound(SOUND_APPEAR)
        me.visible = true
    end
    if countdown(me, 4, true) then
        me:set_action(4)
    end
end

-- Action 4 (0x080BC804): 10 ticks standing; then he plays, or goes off a panel that isn't solid.
local function stand(me: Object)
    if me.phase_init == 0 then
        me.anim = 0
    end
    if countdown(me, 0xA) then
        me:set_action(if field.check(me.panel_x, me.panel_y, SOLID, 0) then 8 else 0xC)
    end
end

-- Action 8, phase 0 (0x080BC854): readying (animation 20); 10 ticks.
local function ready(me: Object)
    if me.phase_init == 0 then
        me.anim = 0x14
    end
    if countdown(me, 0xA) then
        me.phase, me.phase_init = 4, 0
    end
end

-- Action 8, phase 4 (0x080BC876): playing (animation 21, its sound): the notes from the panel ahead; 30 ticks.
local function play(me: Object)
    if me.phase_init == 0 then
        me.anim = 0x15
        local target = find_target(me)
        note.spawn(me, me.panel_x + me.facing, me.panel_y, CHAIN, target)
        battle.play_sound(SOUND_PLAY)
    end
    if countdown(me, 0x1E) then
        me.phase, me.phase_init = 8, 0
    end
end

-- Action 8, phase 8 (0x080BC8B8): 30 ticks standing.
local function rest(me: Object)
    if me.phase_init == 0 then
        me.anim = 0
    end
    if countdown(me, 0x1E) then
        me:set_action(0xC)
    end
end

local ATTACK = { [0] = ready, [4] = play, [8] = rest }

local function attack(me: Object)
    local f = ATTACK[me.phase]
    if not f then
        error(string.format("ToadMan's attack: phase %#x reads past its table (0x080BC848)", me.phase))
    end
    f(me)
end

-- Action 0xC (0x080BC8DE): 4 ticks; then the controller moves on, and he goes.
local function leave(me: Object)
    if me.phase_init == 0 then
        me.anim = 4
    end
    if countdown(me, 4) then
        navi_chips.leave(me, (me.state :: State).controller)
    end
end

local ACTIONS = { [0] = appear, [4] = stand, [8] = attack, [0xC] = leave }

-- 0x080BC76C: his sprite (animation 3, his shadow), his palette by the level, facing his side's way, on his panel,
-- on the ground.
local function init(me: Object)
    local s = me.state :: State
    local sprite = me.sprite
    sprite:load(SPRITE)
    sprite.shadow = "ground"
    me.anim, me.anim_loaded = 3, 3
    sprite:set_animation(3)
    sprite.palette = PALETTES[s.level] or 0
    sprite:set_flip(bit32.bxor(me.alliance, me.flip))
    me:set_coordinates_from_panel()
    local p = me.pos
    me.pos = Vec3.new(p.x, p.y, 0)
    me:set_lifecycle("update")
end

toadman.kind = define.kind {
    id = "bn5:toadman/navi",
    pool = "actor",
    state = { controller = "object", level = "u8" },
    -- 0x080BC748.
    update = function(me: Object)
        local l = me.lifecycle
        if l == "init" then
            init(me)
        elseif l == "update" then
            local action = ACTIONS[me.action]
            if not action then
                error(string.format("ToadMan action %#x reads past his table (0x080BC7C4)", me.action))
            end
            action(me)
        else
            me:free()
        end
        me:update_sprite_while_dimmed()
    end,
}

-- 0x080BC906: ToadMan for `user`, reporting to `controller` (the summon table's entry 13), of a chip of `level`.
function toadman.summon_at(level: number): (user: Object, controller: Object, spec: NaviChipSpec) -> Object?
    return function(user: Object, controller: Object, spec: NaviChipSpec): Object?
        local o = navi_chips.spawn(toadman.kind, user, spec)
        if not o then
            return nil
        end
        local s = o.state :: State
        s.controller = controller
        s.level = level
        return o
    end
end

-- ToadMan's and ToadManSP's (their Param1s 0), ToadManDS's (4).
toadman.summon = toadman.summon_at(0)
toadman.summon_sp = toadman.summon_at(0)
toadman.summon_dark = toadman.summon_at(4)

return toadman
