--!strict
-- A lance (the original's attack object #0x6F, `sub_80D2460`): Lance's
-- three lances (chip.luau) thrust out of the far column's panels, one per
-- row. Each appears 64 pixels past its panel, slides back 8 pixels a tick,
-- hits the panel it is over on its seventh tick, and blinks out over 20
-- ticks. Its sprite never animates. A lance spawned with Param1 set (none
-- of BN6's chips) takes the second palette.

local collision = require("../../rules/collision")
local regions = require("../../lib/regions")
local sparks = require("../../lib/sparks")

local lance = {}

export type State = {
    -- Its palette (0, or 1 with Param1 set).
    palette: number,
}

local SPRITE = asset.sprite("bn6:lance")
local SOUND = asset.sound("bn6:sword-swing")
-- How far past its panel it appears, in pixels, and how fast it slides
-- back, 16.16.
local AHEAD, SPEED = 0x40, 0x8_0000

-- The hit (`dword_80D25C8`): one panel, the wood spark, an attack against
-- attacks, hit modifier 0x51.
local HIT_MOD = 0x51

-- The actions.
local THRUST, BLINK = 0, 4

-- `sub_80D2514`: slide back; the hit on the seventh tick; after eight,
-- blink.
local function thrust(me: Object)
    local p = me.pos
    me.pos = Vec3.new(p.x + me.vel.x, p.y, p.z)
    me:set_panel_from_coordinates()
    if me.timer == 2 then
        battle.hitbox(me, {
            panel_x = me.panel_x,
            panel_y = me.panel_y,
            element = me.element,
            z = me.pos.z,
            region = regions.single,
            hit_effect = sparks.wood,
            target = collision.attack,
            self_type = collision.attack,
            damage = me.damage,
            stamina = me.stamina,
            hit_mod = HIT_MOD,
        })
    end
    local left = me.timer - 1
    me.timer = left
    if left ~= 0 then
        return
    end
    me.timer = 0x14
    me.action = BLINK
    me.phase, me.phase_init = 0, 0
end

-- `sub_80D255A`: blink (hidden every other pair of ticks), then go.
local function blink(me: Object)
    local left = me.timer - 1
    me.timer = left
    if left == 0 then
        me.visible = false
        me:set_lifecycle("destroy")
        return
    end
    if not bit32.btest(left, 2) then
        me.visible = false
    end
end

-- `sub_80D24F0`.
local function tick(me: Object)
    me.visible = true
    local a = me.action
    if a == THRUST then
        thrust(me)
    elseif a == BLINK then
        blink(me)
    else
        error(string.format("lance action %#x reads past its table", a))
    end
end

-- `sub_80D2480`.
local function init(me: Object)
    me.visible = true
    local sprite = me.sprite
    sprite:load(SPRITE)
    me.anim, me.anim_loaded = 0, 0
    sprite:set_animation(0)
    sprite.shadow = "ground"
    sprite:set_flip(bit32.bxor(bit32.bxor(me.alliance, me.flip), 1))
    me:set_coordinates_from_panel()
    local facing = me.facing
    local p = me.pos
    me.pos = Vec3.new(p.x + facing * AHEAD * 0x1_0000, p.y, p.z)
    me.vel = Vec3.new(-facing * SPEED, me.vel.y, me.vel.z)
    me:set_panel_from_coordinates()
    me.timer = 8
    sprite.palette = (me.state :: State).palette
    battle.play_sound(SOUND)
    me:set_lifecycle("update")
    tick(me)
end

lance.kind = define.kind {
    id = "bn6:lance/lance",
    pool = "attack",
    state = { palette = "u8" },
    -- `sub_80D2460`.
    update = function(me: Object)
        local l = me.lifecycle
        if l == "init" then
            init(me)
        elseif l == "update" then
            tick(me)
        else
            me:destroy()
        end
    end,
}

-- `sub_80D2580` and `sub_801155A`: a lance of `owner`'s on (x, y) in
-- palette `palette`, with the damage word `damage`. (It spawns at the
-- registers' values, which its init replaces but for Z.)
function lance.spawn(owner: Object, x: number, y: number, element: number, z: number, palette: number, damage: number): Object?
    local o = battle.spawn(lance.kind, Vec3.new(y, element, z))
    if o then
        o.panel_x, o.panel_y = x, y
        o.element = element
        o:set_damage_word(damage)
        o.alliance, o.flip = owner.alliance, owner.flip
        o.related1 = owner;
        (o.state :: State).palette = palette
    end
    return o
end

-- `sub_80D2596`, instant chip effect 4: the far column, rows 1 to 3, at
-- the user's height.
function lance.instant(user: Object, spec: InstantChipSpec)
    local x = 6 - bit32.bxor(user.alliance, user.flip) * 5
    for y = 1, 3 do
        lance.spawn(user, x, y, spec.element, spec.z, 0, spec.damage)
    end
end

return lance
