--!strict
-- An aura head (attack object #0x92, `sub_80D7400`): AuraHed's and
-- StreamHd's. It charges along its row at its speed from where it was
-- fired, hitting each panel it enters (once per panel: after a hit it is
-- harmless until it leaves that panel), and on its first hit shakes the
-- screen and goes on for a few ticks more. Then it blinks out over 30
-- ticks. What it is (`AuraHead`, types.d.luau) was the original's four
-- parameters: palette, speed (an index into `byte_80D74D4`, whose two
-- entries are both 12 pixels a tick), the stronger hit, and the run to the
-- far edge of the field rather than 3 panels.

local collision = require("../../rules/collision")
local effects = require("../../lib/effects")
local regions = require("@exelib/regions")
local sparks = require("../../lib/sparks")

local head = {}

export type State = {
    -- The panel (x | y << 8) it last hit on, where it keeps its region
    -- off (`+0x60`), and whether it has hit anything yet (`+0x64`).
    hit_panel: number,
    has_hit: boolean,
    -- What it is: its palette, speed (16.16 a tick), stronger hit and run.
    palette: number,
    speed: number,
    strong: boolean,
    far: boolean,
}

-- `sub_8011504`'s words: the sprite (animation 4, a ground shadow), and
-- the collision (a breaking attack, hit modifier 3 or 1 by its strength).
local SPRITE, ANIM = asset.sprite("aura-head"), 4
local STRONG_HIT_MOD, HIT_MOD = 3, 1
local SOUND = asset.sound("set-down")
-- A panel's width, 16.16.
local PANEL = 0x28_0000
-- Panels it charges across unless it runs to the far edge.
local SHORT = 3
local FADE = 0x1E
-- How long the screen shakes on its first hit, and how hard.
local SHAKE, SHAKE_TICKS = 1, 0x1E
-- Its dust, 16 pixels up, when it can't start.
local DUST_Z = 0x10_0000

-- `sub_80D7578` (action 0): charge; out of time, fade.
local function charge(me: Object)
    local p = me.pos
    me.pos = Vec3.new(p.x + me.vel.x, p.y, p.z)
    me:set_panel_from_coordinates()
    me:update_collision_panels()
    local t = me.timer - 1
    me.timer = t
    if t ~= 0 then
        return
    end
    me.timer = FADE
    me.collision:set_region(nil)
    me.action = 4
    me.phase = 0
    me.phase_init = 0
end

-- `sub_80D75A4` (action 4): blink out; gone off the field.
local function fade(me: Object)
    if not field.valid(me.panel_x, me.panel_y) then
        me:set_lifecycle("destroy")
        return
    end
    me.visible = bit32.btest(me.timer, 2)
    local t = me.timer - 1
    me.timer = t
    if t == 0 then
        me:set_lifecycle("destroy")
    end
end

-- `sub_80D74DC`: resolve its hits; the first shakes the screen and cuts
-- its run short.
local function tick(me: Object)
    local c = me.collision
    local s = me.state :: State
    c:remove()
    me:hit_spark()
    if c.hit_flags ~= 0 then
        s.hit_panel = me.panel_x + me.panel_y * 0x100
        c:set_region(nil)
        if not s.has_hit then
            me.timer = int.tdiv(PANEL, me.vel.y) + 2
            me.timer2 = 0
            battle.shake_camera(SHAKE, SHAKE_TICKS)
            s.has_hit = true
        end
    end
    local a = me.action
    if a == 0 then
        charge(me)
    elseif a == 4 then
        fade(me)
    else
        error(string.format("aura head action %#x reads past its table", a))
    end
    local highlight = true
    if c.region == nil and me.action ~= 4 and me.lifecycle ~= "destroy" then
        if me.panel_x + me.panel_y * 0x100 == s.hit_panel then
            highlight = false
        else
            c:set_region(regions.single)
        end
    end
    if highlight then
        field.highlight(me.panel_x, me.panel_y)
    end
    me:update_sprite()
    c:present()
end

-- Off the field or without collision: a puff of dust 16 pixels up, and
-- gone.
local function fizzle(me: Object)
    local p = me.pos
    battle.effect(Vec3.new(p.x, p.y, p.z + DUST_Z), effects.puff)
    me:free()
end

-- `sub_80D7420`.
local function init(me: Object)
    local s = me.state :: State
    me:set_coordinates_from_panel()
    if not field.valid(me.panel_x, me.panel_y) then
        return fizzle(me)
    end
    -- `sub_8011504`.
    local sprite = me.sprite
    sprite:load(SPRITE)
    sprite:set_animation(0)
    sprite.shadow = "ground"
    me.anim = ANIM
    me.anim_loaded = 0xFF
    if not me:create_collision() then
        return fizzle(me)
    end
    me:setup_collision(collision.breaking, collision.hits_navis, if s.strong then STRONG_HIT_MOD else HIT_MOD)
    sprite:set_flip(bit32.bxor(me.alliance, me.flip))
    me.visible = true
    sprite.palette = s.palette
    local speed = s.speed
    local vx = speed * me.facing
    me.vel = Vec3.new(vx, speed, me.vel.z)
    local p = me.pos
    me.pos = Vec3.new(p.x + vx * 2, p.y, p.z)
    me.collision:set_hit_effect(sparks.breaking)
    local panels = SHORT
    if s.far then
        -- To the far edge: column 7 for side 0, column 0 for side 1.
        panels = math.abs(bit32.bxor(me.alliance, 1) * 7 - me.panel_x)
    end
    me.timer = int.tdiv(PANEL * panels - speed - speed, speed) + 1
    me.collision:present()
    battle.play_sound(SOUND)
    me.lifecycle = "update"
    tick(me)
end

head.kind = new.kind {
    id = "aurahed/head",
    pool = "attack",
    state = {
        hit_panel = "u32",
        has_hit = "bool",
        palette = "u8",
        speed = "i32",
        strong = "bool",
        far = "bool",
    },
    -- `sub_80D7400`.
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

-- `sub_80D75D4`: `what` for `user` on (x, y), with the damage word
-- `damage`. It starts 12 pixels down (the spawner's Z register).
function head.spawn(user: Object, x: number, y: number, element: number, what: AuraHead, damage: number): Object?
    local o = battle.spawn(head.kind, Vec3.new(y, element, -0xC_0000))
    if o then
        o.panel_x, o.panel_y = x, y
        o.element = element
        o:set_damage_word(damage)
        o.alliance, o.flip = user.alliance, user.flip
        o.related1 = user
        local s = o.state :: State
        s.palette = what.palette
        s.speed = what.speed
        s.strong = what.strong
        s.far = what.far
    end
    return o
end

return head
