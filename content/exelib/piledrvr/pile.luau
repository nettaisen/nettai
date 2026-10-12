--!strict
-- The pile driver's pile, as EXE4 and EXE5 share it (EXE5's attack object #0x99, 0x080D9C1C; EXE4's #0x99,
-- 0x080E0318, the same code; EXE6's code, `sub_80D8620`, which no EXE6 chip uses): each game makes its own kind
-- (`pile.make_kind`, a `Look`). Set down on its panel with a sound, it stands 68 ticks; then, if it saw someone ahead,
-- it sends its beam (./beam) from the panel past it and works for 151 ticks (animation 2) and 7 more (animation 3).
-- Then, or at once with no one ahead, its controller (./controller) is told it's done, and it goes. It runs while
-- dimmed. (The second pile is turned around: EXE5's controller flips it; EXE4's pile and beam, with no flip to read,
-- turn by their first parameter, which pile it is. Here both are the object's flip.)

local beam = require("./beam")

export type State = {
    -- Its controller (+0x4C: the controller's flag byte it clears when done, +0x06 or +0x07).
    controller: Object?,
    -- Which of the controller's piles it is (0: the first, its +0x06; 1: the second, its +0x07).
    slot: number,
    -- How far past the panel ahead of it the one it beams at stands (+0x05); 0xFF: no one.
    distance: number,
}

-- A game's pile: its kind's key, its sprite, the sound as it is set down, and its beam's kind.
export type Look = { id: string, sprite: SpriteAsset, sound: Sound, beam: Kind }

-- The controller's flags its piles clear (./controller's state).
type Waiting = { first_busy: boolean, second_busy: boolean }

local pile = {}

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

-- A game's pile kind.
function pile.make_kind(look: Look): Kind
    -- Action 0: 7 ticks, the sound on the first.
    local function set_down(me: Object)
        if me.phase_init == 0 then
            me.timer = SET_DOWN
            battle.play_sound(look.sound)
            me.phase_init = 4
        end
        if count_down(me) then
            me:set_action(4)
        end
    end

    -- Action 4: 61 ticks; then its beam, or with no one ahead it is done.
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

    -- Action 8: its beam from the panel ahead, at the same damage word, and animation 2, for 151 ticks.
    local function work(me: Object)
        if me.phase_init == 0 then
            me.timer = WORK
            local word = bit32.bor(me.damage, bit32.lshift(me.stamina, 16))
            beam.spawn(look.beam, me, me.panel_x + me.facing, me.panel_y, me.element, word, (me.state :: State).distance)
            me.anim = 2
            me.phase_init = 4
        end
        if count_down(me) then
            me:set_action(0x0C)
        end
    end

    -- Action 0x0C: animation 3 for 7 ticks; then it is done.
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

    local function tick(me: Object)
        local action = ACTIONS[me.action]
        if not action then
            error(string.format("the pile driver's pile: action %#x reads past its table", me.action))
        end
        action(me)
    end

    -- Its sprite (its first part drawn with it), shown, facing its way, on its panel on the ground; its first update.
    -- Animation 0 is asked for, to be loaded by the update that follows. What the init itself loads and steps is the
    -- 0xFF left in the register (EXE5's 0x080D9C5C, EXE4's 0x080E0358), an animation the sheet doesn't have: no palette
    -- offset. A game whose dimmed update loads alone (EXE4's) so shows the pile a tick in its own colors before its
    -- first frame's white.
    local function init(me: Object)
        local sprite = me.sprite
        sprite:load(look.sprite)
        -- sprite_noShadow: the first part is drawn with the sprite.
        sprite.shadow = "with_sprite"
        me.anim, me.anim_loaded = 0, 0xFF
        sprite:set_animation(0xFF)
        sprite:step()
        me.visible = true
        sprite:set_flip(bit32.bxor(me.alliance, me.flip))
        me:set_coordinates_from_panel()
        me.pos = Vec3.new(me.pos.x, me.pos.y, 0)
        me:set_lifecycle("update")
        tick(me)
    end

    return new.kind {
        id = look.id,
        pool = "attack",
        state = { controller = "object", slot = "u8", distance = "u8" },
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
end

-- Pile `slot` (of `kind`) of `controller` on (x, y) with `element` and the damage word, seeing someone `distance`
-- panels past the panel ahead of it (NONE: no one), running while dimmed. (Its position is the spawner's registers:
-- the row, the element and the bonus, until its init places it.)
function pile.spawn(
    kind: Kind,
    controller: Object,
    slot: number,
    x: number,
    y: number,
    element: number,
    bonus: number,
    damage: number,
    distance: number
): Object?
    local o = battle.spawn(kind, Vec3.new(y, element, bonus))
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
