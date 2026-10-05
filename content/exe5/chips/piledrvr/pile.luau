--!strict
-- PileDrvr's pile (EXE5's attack object #0x99, 0x080D9C1C; EXE6's code, `sub_80D8620`, which no EXE6 chip uses): set
-- down on its panel with a sound (sprite 0C-46), it stands 68 ticks; then, if it saw someone ahead, it sends its beam
-- (./beam) from the panel past it and works for 151 ticks (animation 2) and 7 more (animation 3). Then, or at once
-- with no one ahead, its controller (./controller) is told it's done, and it goes. It runs while dimmed.

local beam = require("./beam")

export type State = {
    -- Its controller (+0x4C: the controller's flag byte it clears when done, +0x06 or +0x07).
    controller: Object?,
    -- Which of the controller's piles it is (0: the first, its +0x06; 1: the second, its +0x07).
    slot: number,
    -- How far past the panel ahead of it the one it beams at stands (+0x05); 0xFF: no one.
    distance: number,
}

-- The controller's flags its piles clear (./controller's state).
type Waiting = { first_busy: boolean, second_busy: boolean }

local pile = {}

local SPRITE = asset.sprite("pile-driver")
local SOUND = asset.sound("pile-driver")
local NONE = 0xFF
-- How long it stands, works, and winds down (each one tick more: to -1).
local SET_DOWN, STAND, WORK, WIND_DOWN = 6, 0x3C, 0x96, 6

-- Its controller's flag cleared: this pile is done.
local function release(me: Object)
    local s = me.state :: State
    local c = s.controller
    if c then
        local w = c.state :: Waiting
        if s.slot == 0 then
            w.first_busy = false
        else
            w.second_busy = false
        end
    end
end

-- The timer's count down (a u16, read signed as the code does): whether it ran out.
local function count_down(me: Object): boolean
    local left = me.timer - 1
    me.timer = int.u16(left)
    return left < 0
end

-- Action 0 (0x080D9CAC): 7 ticks, the sound on the first.
local function set_down(me: Object)
    if me.phase_init == 0 then
        me.timer = SET_DOWN
        battle.play_sound(SOUND)
        me.phase_init = 4
    end
    if count_down(me) then
        me:set_action(4)
    end
end

-- Action 4 (0x080D9CD6): 61 ticks; then its beam, or with no one ahead it is done.
local function stand(me: Object)
    if me.phase_init == 0 then
        me.timer = STAND
        me.phase_init = 4
    end
    if count_down(me) then
        if (me.state :: State).distance == NONE then
            release(me)
            me:set_lifecycle("destroy")
            return
        end
        me:set_action(8)
    end
end

-- Action 8 (0x080D9D0A): its beam from the panel ahead, at the same damage word, and animation 2, for 151 ticks.
local function work(me: Object)
    if me.phase_init == 0 then
        me.timer = WORK
        local word = bit32.bor(me.damage, bit32.lshift(me.stamina, 16))
        beam.spawn(me, me.panel_x + me.facing, me.panel_y, me.element, word, (me.state :: State).distance)
        me.anim = 2
        me.phase_init = 4
    end
    if count_down(me) then
        me:set_action(0x0C)
    end
end

-- Action 0x0C (0x080D9D4A): animation 3 for 7 ticks; then it is done.
local function wind_down(me: Object)
    if me.phase_init == 0 then
        me.timer = WIND_DOWN
        me.anim = 3
        me.phase_init = 4
    end
    if count_down(me) then
        release(me)
        me:set_lifecycle("destroy")
    end
end

local ACTIONS = { [0] = set_down, [4] = stand, [8] = work, [0x0C] = wind_down }

-- 0x080D9C8A.
local function tick(me: Object)
    local action = ACTIONS[me.action]
    if not action then
        error(string.format("PileDrvr's pile: action %#x reads past its table (0x080D9C9C)", me.action))
    end
    action(me)
end

-- 0x080D9C40: its sprite (animation 0, a frame in, its first part drawn with it), shown, facing its way, on its
-- panel on the ground; its first update.
local function init(me: Object)
    local sprite = me.sprite
    sprite:load(SPRITE)
    -- sprite_noShadow: the first part is drawn with the sprite.
    sprite.shadow = "with_sprite"
    me.anim, me.anim_loaded = 0, 0xFF
    sprite:set_animation(0)
    sprite:step()
    me.visible = true
    sprite:set_flip(bit32.bxor(me.alliance, me.flip))
    me:set_coordinates_from_panel()
    me.pos = Vec3.new(me.pos.x, me.pos.y, 0)
    me:set_lifecycle("update")
    tick(me)
end

pile.kind = define.kind {
    id = "piledrvr/pile",
    pool = "attack",
    state = { controller = "object", slot = "u8", distance = "u8" },
    -- 0x080D9C1C.
    update = function(me: Object)
        local l = me.lifecycle
        if l == "init" then
            init(me)
        elseif l == "update" then
            tick(me)
        else
            me:destroy()
            return
        end
        me:update_sprite_while_dimmed()
    end,
}

-- 0x080D9D72: pile `slot` of `controller` on (x, y) with `element` and the damage word, seeing someone `distance`
-- panels past the panel ahead of it (NONE: no one), running while dimmed. (Its position is the spawner's registers:
-- the row, the element and the bonus, until its init places it.)
function pile.spawn(
    controller: Object,
    slot: number,
    x: number,
    y: number,
    element: number,
    bonus: number,
    damage: number,
    distance: number
): Object?
    local o = battle.spawn(pile.kind, Vec3.new(y, element, bonus))
    if o then
        o.panel_x, o.panel_y = x, y
        o.element = element
        o:set_damage_word(damage)
        o.alliance, o.flip = controller.alliance, controller.flip
        local s = o.state :: State
        s.controller, s.slot, s.distance = controller, slot, distance
        o.run_while_dimmed = true
    end
    return o
end

return pile
