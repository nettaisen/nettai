--!strict
-- PileDrvr's beam (BN5's attack object #0x9A, 0x080D9DA0; BN6's code, `sub_80D879C`, which no BN6 chip uses): from
-- the panel ahead of its pile, as long as the way to the one its pile saw (sprite 0C-4A, its animation by that
-- distance, drawn 60 pixels ahead), it plays its opening animation through; then it hits that one's panel ten times,
-- 16 ticks apart (0x080D9E6C), and goes. Its sound every 16 ticks; it runs while dimmed.

local collision = require("../../rules/collision")
local regions = require("@common/regions")
local sparks = require("../../lib/sparks")

export type State = {
    -- How many panels past its own the one it hits stands (+0x05).
    distance: number,
    -- Its ticks, for its sound (+0x07).
    ticks: number,
}

local beam = {}

local SPRITE = asset.sprite("bn5:pile-beam")
local SOUND = asset.sound("bn5:django-charge")
-- Where it is drawn, ahead of its panel (in pixels), and the height it is spawned at (16.16; it keeps it).
local DRAWN_AHEAD, HEIGHT = 0x3C, 0x1C_0000
-- Its hits: ten, 16 ticks apart (the timer to -1), the first at once.
local HITS, BETWEEN = 0x0A, 0x0F
local HIT_MOD = 3

-- 0x080D9E50 (action 0): its opening animation to its last frame; then the next and the hits.
local function open(me: Object)
    if bit32.btest(me.sprite.frame_flags, 0x80) then
        me.anim = int.u8(me.anim + 1)
        me:set_action(4)
    end
end

-- 0x080C4DC6 (`sub_80C53A6`): a hit on the panel `distance` ahead (region 1, the erase spark, hitting navis,
-- piercing and breaking, hit modifier 3) that runs while dimmed.
local function hit(me: Object)
    local box = battle.hitbox(me, {
        panel_x = me.panel_x + me.facing * (me.state :: State).distance,
        panel_y = me.panel_y,
        element = me.element,
        z = me.pos.z,
        region = regions.single,
        hit_effect = sparks.erase,
        target = collision.hits_navis,
        self_type = collision.piercing_break,
        damage = me.damage,
        stamina = me.stamina,
        hit_mod = HIT_MOD,
    })
    if box then
        box.run_while_dimmed = true
    end
end

-- 0x080D9E6C (action 4): a hit at once and every 16 ticks, ten in all; then it goes.
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

-- 0x080D9E20: its action; then its sound on the first of every 16 ticks.
local function tick(me: Object)
    local action = ACTIONS[me.action]
    if not action then
        error(string.format("PileDrvr's beam: action %#x reads past its table (0x080D9E48)", me.action))
    end
    action(me)
    local s = me.state :: State
    s.ticks = int.u8(s.ticks + 1)
    if bit32.band(s.ticks, 0x0F) == 1 then
        battle.play_sound(SOUND)
    end
end

-- 0x080D9DC4: its sprite (animation twice its distance, a frame in, its first part drawn with it), shown, facing its
-- way, on its panel and drawn 60 pixels ahead; its first update.
local function init(me: Object)
    local s = me.state :: State
    local sprite = me.sprite
    sprite:load(SPRITE)
    -- sprite_noShadow: the first part is drawn with the sprite.
    sprite.shadow = "with_sprite"
    local anim = int.u8(s.distance * 2)
    me.anim, me.anim_loaded = anim, 0xFF
    sprite:set_animation(anim)
    sprite:step()
    me.visible = true
    sprite:set_flip(bit32.bxor(me.alliance, me.flip))
    me:set_coordinates_from_panel()
    me.pos = Vec3.new(int.i32(me.pos.x + me.facing * DRAWN_AHEAD * 0x10000), me.pos.y, me.pos.z)
    s.ticks = 0
    me:set_lifecycle("update")
    tick(me)
end

beam.kind = define.kind {
    id = "bn5:piledrvr/beam",
    pool = "attack",
    state = { distance = "u8", ticks = "u8" },
    -- 0x080D9DA0.
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

-- 0x080D9EBC: `pile`'s beam on (x, y) with `element` and the damage word, hitting `distance` panels past its own,
-- running while dimmed. (Its position is the spawner's registers, the row, the element and its height, until its
-- init places it; the height stays.)
function beam.spawn(pile: Object, x: number, y: number, element: number, damage: number, distance: number): Object?
    local o = battle.spawn(beam.kind, Vec3.new(y, element, HEIGHT))
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
