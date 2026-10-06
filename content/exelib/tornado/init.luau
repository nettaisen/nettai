--!strict
-- A tornado, as EXE5 and EXE6 share it (EXE6's attack object #0x31,
-- `sub_80C9F78`; EXE5's #0x31, 0x080CAC60): Tornado's
-- and Static's. It spins on its panel for its time, its region re-armed
-- every few ticks (by its variant) after each hit, so it hits what stands
-- there again and again. A plain tornado on a volcano, grass or ice panel
-- takes the panel's variant instead, turns the panel normal and doubles
-- its damage. It may inflict a status; the HP-bug variant carries the HP
-- bug. It ends when time is up in the battle. Each game makes its kind with
-- `tornado.make_kind(id, look)` (its sprite and sound, the panels' variants,
-- the doubling, its collision types and sparks) and its variants with
-- `tornado.variant`.

local regions = require("./regions")

local tornado = {}

-- A tornado's look and rhythm (`TornadoVariant`, types.d.luau).
export type Variant = TornadoVariant

export type State = {
    variant: Variant,
    -- The status its hits inflict (nil: none).
    status: Status?,
    -- How much of the re-arm count one tick uses (ExtraVars+0), and the
    -- count (ExtraVars+4).
    rearm_step: number,
    rearm_ticks: number,
}

-- What a tornado is spawned with (`TornadoSpec`, types.d.luau): its
-- variant, the status its hits inflict (nil: none; the original's second
-- parameter) and its ticks (the third and fourth).
export type Spec = TornadoSpec

-- A tornado variant.
function tornado.variant(v: Variant): Variant
    return new.record("tornado-variant", v)
end

-- The damage bits that are flags, kept when the damage doubles.
local DAMAGE_FLAGS = 0xF800

-- The destroy state, hidden and harmless.
local function finish(me: Object)
    me.collision:set_region(nil)
    me.visible = false
    me:set_lifecycle("destroy")
end

-- `sub_80CA0B0`: count down; re-arm the region every so many ticks.
local function spin(me: Object)
    local t = me.timer - 1
    me.timer = t
    if t == 0 then
        finish(me)
        return
    end
    local s = me.state :: State
    local left = me.timer2 - s.rearm_step
    me.timer2 = left
    if left > 0 then
        return
    end
    me.timer2 = s.rearm_ticks
    me.collision:set_region(regions.single)
end

-- `sub_80CA074`: resolve its hits (a hit takes its region away), spin.
local function tick(me: Object)
    local c = me.collision
    c:remove()
    me:hit_spark()
    if battle.time_up() then
        me.visible = false
        c:set_region(nil)
        me:set_lifecycle("destroy")
        return
    end
    if c.hit_flags ~= 0 then
        c:set_region(nil)
    end
    spin(me)
    me:update_sprite()
    c:present()
end

-- `sub_80CA234`: the variant a plain tornado takes from its panel (none
-- when it stays plain).
local function panel_variant(me: Object, look: TornadoLook): Variant?
    local p = field.panel(me.panel_x, me.panel_y)
    -- (Off the field the game reads a type from address 0 and the tornado
    -- frees itself right after, whatever it found.)
    return p and look.panel_variants[p.kind]
end

-- `sub_80C9F98`: take the panel's variant, then spin on it.
local function init(me: Object, look: TornadoLook)
    local s = me.state :: State
    if s.variant.plain then
        local v = panel_variant(me, look)
        if v then
            s.variant = v
            field.set_type(me.panel_x, me.panel_y, "normal")
            if look.doubles_on_panel then
                local d = me.damage
                me.damage = bit32.bor(bit32.band(d, DAMAGE_FLAGS), bit32.band(d, bit32.bnot(DAMAGE_FLAGS)) * 2)
            end
        end
    end
    if not field.valid(me.panel_x, me.panel_y) then
        me:free()
        return
    end
    me.visible = true
    local sprite = me.sprite
    sprite:load(look.sprite)
    sprite.shadow = "with_sprite"
    me.anim = 0
    me.anim_loaded = 0
    sprite:set_animation(0)
    sprite:set_flip(bit32.bxor(me.alliance, me.flip))
    s.rearm_step = 1
    me:set_coordinates_from_panel()
    local p = me.pos
    me.pos = Vec3.new(p.x, p.y, 0)
    local v = s.variant
    s.rearm_ticks = v.rearm
    me.timer2 = v.rearm
    sprite.palette = v.palette
    if not me:create_collision() then
        me:free()
        return
    end
    me:setup_collision(if v.bug and look.bug_is then look.bug_is else look.is, look.target, 1)
    local c = me.collision
    c:set_hit_effect(look.spark(me.element))
    local status = s.status
    if status then
        c.status_base = status
    end
    if v.bug then
        c.bugs = v.bug
    end
    c:present()
    battle.play_sound(look.sound)
    me:set_lifecycle("update")
    tick(me)
end

-- A tornado kind (`id`) of `look`.
function tornado.make_kind(id: string, look: TornadoLook): Kind
    return new.kind {
        id = id,
        pool = "attack",
        state = { variant = "record:tornado-variant", status = "status", rearm_step = "u32", rearm_ticks = "u32" },
        -- `sub_80C9F78`.
        update = function(me: Object)
            local l = me.lifecycle
            if l == "init" then
                init(me, look)
            elseif l == "update" then
                tick(me)
            else
                me:destroy()
            end
        end,
    }
end

-- `sub_80CA0E2`: a tornado (of `kind`, a game's) for `owner`'s side on
-- (x, y) with the damage word `damage`. Its position is the spawner's
-- registers (the panel Y, the element and 0) until its init places it.
function tornado.spawn(kind: Kind, owner: Object, x: number, y: number, element: number, damage: number, spec: Spec): Object?
    local o = battle.spawn(kind, Vec3.new(y, element, 0))
    if o then
        local s = o.state :: State
        s.variant, s.status = spec.variant, spec.status
        o.element = element
        o.panel_x, o.panel_y = x, y
        o.timer = spec.ticks
        o:set_damage_word(damage)
        o.alliance = owner.alliance
    end
    return o
end

return tornado
