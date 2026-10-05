--!strict
-- NumTrap's press (EXE5's attack object #0x8C, 0x080D7CE4; EXE5's own), which NumberMan's sprung trap (./trap)
-- brings down on four panels: its own, the one right of it, and the two below those. It hangs 192 pixels over
-- their middle, its shadow on the ground, waits (by its variant: the trap's 5 ticks), falls (the trap's 32 pixels a
-- tick) and lands: the camera shakes, and each of the four panels takes a hit (an attack at navis, hit modifier 3,
-- the plain spark) of its damage and element. 40 ticks on it is gone. It stands still while dimmed, and goes when the battle is over.
--
-- Its variants 0 to 3 (20 ticks, 16 pixels a tick) are the other trap's (attack object #0x8B, the operation
-- battle's NumberMan's). In the operation battle it marks its panels as it waits and falls (lib/navi-chips/marker).

local collision = require("../../rules/collision")
local marker = require("../../lib/navi-chips/marker")
local regions = require("@exelib/regions")
local sparks = require("../../lib/sparks")

local press = {}

export type State = {
    -- Its variant (Param0), which picks its wait and its speed.
    variant: number,
}

local SPRITE = asset.sprite("numtrap-press")
local SOUND_FALL = asset.sound("drop")
local SOUND_LAND = asset.sound("drumroll")
-- Where it hangs from its panel's middle, in pixels: over the middle of the four panels, and up.
local RIGHT, DOWN, UP = 20, 12, 192
-- By its variant: its wait (0x080D7DC0) and its fall a tick (0x080D7E10).
local WAIT: { [number]: number } = { [0] = 20, 20, 20, 20, 5 }
local FALL: { [number]: number } = { [0] = 0x10_0000, 0x10_0000, 0x10_0000, 0x10_0000, 0x20_0000 }
-- The four panels it lands on, from its own (0x080D7E88).
local PANELS = { { 0, 0 }, { 0, 1 }, { 1, 0 }, { 1, 1 } }
local HIT_MOD = 3
local GONE_TICKS = 40

-- Its actions.
local WAITING, FALLING, LANDED = 0x0, 0x4, 0x8

-- 0x080D7D8C: it hangs for its variant's ticks. (0x080D7EC0: the operation battle's marks on its two columns, in
-- the top row for a press in it and the bottom row for any other.)
local function wait(me: Object)
    if me.phase == 0 then
        me.phase = 4
        me.timer = WAIT[(me.state :: State).variant]
        return
    end
    local row = if me.panel_y == 1 then 1 else 3
    marker.spawn(me, me.panel_x, row, 1, 1, false)
    marker.spawn(me, me.panel_x + 1, row, 1, 1, false)
    local left = me.timer - 1
    me.timer = int.u16(left)
    if left < 0 then
        me:set_action(FALLING)
    end
end

-- 0x080D7DCA: it falls at its variant's speed until it is down. (0x080D7EEC: the operation battle's marks on its
-- four panels.)
local function fall(me: Object)
    if me.phase == 0 then
        me.phase = 4
        battle.play_sound(SOUND_FALL)
        me.vel = Vec3.new(me.vel.x, me.vel.y, FALL[(me.state :: State).variant])
        return
    end
    for _, d in PANELS do
        marker.spawn(me, me.panel_x + d[1], me.panel_y + d[2], 1, 1, false)
    end
    local p = me.pos
    local z = int.i32(p.z - me.vel.z)
    if z > 0 then
        me.pos = Vec3.new(p.x, p.y, z)
        return
    end
    me.pos = Vec3.new(p.x, p.y, 0)
    me:set_action(LANDED)
end

-- 0x080D7E24: it lands: the shake, a hit on each of its four panels; 40 ticks, and it is gone.
local function land(me: Object)
    if me.phase == 0 then
        me.phase = 4
        battle.play_sound(SOUND_LAND)
        battle.shake_camera(2, 20)
        for _, d in PANELS do
            battle.hitbox(me, {
                panel_x = me.panel_x + d[1],
                panel_y = me.panel_y + d[2],
                element = me.element,
                z = 0,
                region = regions.single,
                hit_effect = sparks.plain,
                target = collision.hits_navis,
                self_type = collision.attack,
                damage = me.damage,
                stamina = me.stamina,
                hit_mod = HIT_MOD,
            })
        end
        me.timer = GONE_TICKS
        return
    end
    local left = me.timer - 1
    me.timer = int.u16(left)
    if left < 0 then
        me.lifecycle = "destroy"
    end
end

-- 0x080D7D08: over the middle of its four panels, 192 pixels up; its sprite with its shadow on the ground, shown,
-- facing its side's way.
local function init(me: Object)
    local s = me.state :: State
    if WAIT[s.variant] == nil then
        error(string.format("NumTrap's press: variant %d reads past its tables (0x080D7DC0, 0x080D7E10)", s.variant))
    end
    me:set_coordinates_from_panel()
    local p = me.pos
    me.pos = Vec3.new(int.i32(p.x + RIGHT * 0x1_0000), int.i32(p.y + DOWN * 0x1_0000), UP * 0x1_0000)
    local sprite = me.sprite
    sprite:load(SPRITE)
    sprite.shadow = "ground"
    me.visible = true
    me.anim, me.anim_loaded = 0, 0
    sprite:set_animation(0)
    sprite.palette = 0
    sprite:set_flip(bit32.bxor(me.alliance, me.flip))
    me:set_lifecycle("update")
end

-- 0x080D7D5A.
local function tick(me: Object)
    if battle.over() then
        me.lifecycle = "destroy"
        return
    end
    if battle.dimmed() then
        return
    end
    local a = me.action
    if a == WAITING then
        wait(me)
    elseif a == FALLING then
        fall(me)
    elseif a == LANDED then
        land(me)
    else
        error(string.format("NumTrap's press: action %#x reads past its table (0x080D7D80)", a))
    end
end

press.kind = define.kind {
    id = "numberman/press",
    pool = "attack",
    state = { variant = "u8" },
    -- 0x080D7CE4
    update = function(me: Object)
        local l = me.lifecycle
        if l == "init" then
            init(me)
        elseif l == "update" then
            tick(me)
        else
            me:destroy()
        end
        me:update_sprite()
    end,
}

-- 0x080D7EA0: a press of `owner`'s side over the four panels from (x, y), of `variant`, with the element and the
-- damage word.
function press.spawn(owner: Object, x: number, y: number, element: number, variant: number, damage: number): Object?
    local o = battle.spawn(press.kind, Vec3.zero)
    if o then
        (o.state :: State).variant = variant
        o.panel_x, o.panel_y = x, y
        o.element = element
        o.related1 = owner
        o.alliance = owner.alliance
        o:set_damage_word(damage)
    end
    return o
end

return press
