--!strict
-- The pile driver's beam, as EXE4 and EXE5 share it (EXE5's attack object #0x9A, 0x080D9DA0; EXE4's #0x9A,
-- 0x080E04A4, the same code; EXE6's code, `sub_80D879C`, which no EXE6 chip uses): each game makes its own kind
-- (`beam.make_kind`, a `Look`). From the panel ahead of its pile, as long as the way to the one its pile saw (its
-- animation by that distance, drawn 60 pixels ahead), it plays its opening animation through; then it hits that
-- one's panel ten times, 16 ticks apart, and goes. Its sound every 16 ticks; it runs while dimmed.

local regions = require("../regions")

export type State = {
    -- How many panels past its own the one it hits stands (+0x05).
    distance: number,
    -- Its ticks, for its sound (+0x07).
    ticks: number,
}

-- A game's beam: its kind's key, its sprite and sound, its hits' spark, and what they are and hit.
export type Look = {
    id: string,
    sprite: SpriteAsset,
    sound: Sound,
    spark: Spark,
    is: CollisionType,
    hits: CollisionType,
}

local beam = {}

-- Where it is drawn, ahead of its panel (in pixels), and the height it is spawned at (16.16; it keeps it).
local DRAWN_AHEAD, HEIGHT = 0x3C, 0x1C_0000
-- Its hits: ten, 16 ticks apart (the timer to -1), the first at once.
local HITS, BETWEEN = 0x0A, 0x0F
local HIT_MOD = 3

-- Action 0: its opening animation to its last frame; then the next and the hits. (On its first tick the sprite still
-- holds what its init loaded, the animation 0xFF the sheet doesn't have, where the original reads no last frame.)
local function open(me: Object)
    if me.anim_loaded ~= me.anim then
        return
    end
    if bit32.btest(me.sprite.frame_flags, 0x80) then
        me.anim = int.u8(me.anim + 1)
        me:set_action(4)
    end
end

-- A game's beam kind.
function beam.make_kind(look: Look): Kind
    -- `sub_80C53A6` (EXE5's 0x080C4DC6, EXE4's 0x080CD81E): a hit on the panel `distance` ahead (region 1, the look's
    -- spark and types, hit modifier 3) that runs while dimmed.
    local function hit(me: Object)
        local box = battle.hitbox(me, {
            panel_x = me.panel_x + me.facing * (me.state :: State).distance,
            panel_y = me.panel_y,
            element = me.element,
            z = me.pos.z,
            region = regions.single,
            hit_effect = look.spark,
            target = look.hits,
            self_type = look.is,
            damage = me.damage,
            stamina = me.stamina,
            hit_mod = HIT_MOD,
        })
        if box then
            box.run_while_dimmed = true
        end
    end

    -- Action 4: a hit at once and every 16 ticks, ten in all; then it goes.
    local function hits(me: Object)
        if me.phase_init == 0 then
            me.timer = 0
            me.timer2 = HITS
            me.phase_init = 4
        end
        local left = me.timer - 1
        me.timer = int.u16(left)
        if left >= 0 then
            return
        end
        me.timer = BETWEEN
        hit(me)
        local n = me.timer2 - 1
        me.timer2 = int.u16(n)
        if n <= 0 then
            me:set_lifecycle("destroy")
        end
    end

    local ACTIONS = { [0] = open, [4] = hits }

    -- Its action; then its sound on the first of every 16 ticks.
    local function tick(me: Object)
        local action = ACTIONS[me.action]
        if not action then
            error(string.format("the pile driver's beam: action %#x reads past its table", me.action))
        end
        action(me)
        local s = me.state :: State
        s.ticks = int.u8(s.ticks + 1)
        if bit32.band(s.ticks, 0x0F) == 1 then
            battle.play_sound(look.sound)
        end
    end

    -- Its sprite (its first part drawn with it), shown, facing its way, on its panel and drawn 60 pixels ahead; its
    -- first update. The animation twice its distance is asked for, to be loaded by the update that follows; what the
    -- init itself loads and steps is the 0xFF left in the register (EXE5's 0x080D9DE2, EXE4's 0x080E04E6), as the
    -- pile's does.
    local function init(me: Object)
        local s = me.state :: State
        local sprite = me.sprite
        sprite:load(look.sprite)
        -- sprite_noShadow: the first part is drawn with the sprite.
        sprite.shadow = "with_sprite"
        me.anim, me.anim_loaded = int.u8(s.distance * 2), 0xFF
        sprite:set_animation(0xFF)
        sprite:step()
        me.visible = true
        sprite:set_flip(bit32.bxor(me.alliance, me.flip))
        me:set_coordinates_from_panel()
        me.pos = Vec3.new(int.i32(me.pos.x + me.facing * DRAWN_AHEAD * 0x10000), me.pos.y, me.pos.z)
        s.ticks = 0
        me:set_lifecycle("update")
        tick(me)
    end

    return new.kind {
        id = look.id,
        pool = "attack",
        state = { distance = "u8", ticks = "u8" },
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

-- `pile`'s beam (of `kind`) on (x, y) with `element` and the damage word, hitting `distance` panels past its own,
-- running while dimmed, facing as its pile does. (Its position is the spawner's registers, the row, the element and
-- its height, until its init places it; the height stays.)
function beam.spawn(kind: Kind, pile: Object, x: number, y: number, element: number, damage: number, distance: number): Object?
    local o = battle.spawn(kind, Vec3.new(y, element, HEIGHT))
    if o then
        o.panel_x, o.panel_y = x, y
        o.element = element
        o:set_damage_word(damage)
        o.alliance, o.flip = pile.alliance, pile.flip
        local s = o.state :: State
        s.distance = distance
        o.run_while_dimmed = true
    end
    return o
end

return beam
