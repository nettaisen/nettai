--!strict
-- A tornado (attack object #0x31, `sub_80C9F78`): Tornado's and Static's.
-- It spins on its panel for its timer (the third and fourth parameters),
-- its region re-armed every few ticks (by kind) after each hit, so it hits
-- what stands there again and again. Its first parameter is its kind: a
-- plain tornado (1) on a volcano, grass or ice panel takes the panel's
-- element instead (kinds 4, 0 and 6), turns the panel normal and doubles
-- its damage. The second parameter is a status it inflicts; kind 3 carries
-- the HP bug. It ends when time is up in the battle.

local tornado = {}

export type State = {
    -- How much of the re-arm count one tick uses (ExtraVars+0), and the
    -- count (ExtraVars+4).
    rearm_step: number,
    rearm_ticks: number,
}

tornado.state = { rearm_step = "u32", rearm_ticks = "u32" }

local SPRITE = "0c-17"
-- By kind (`byte_80CA064`): the palette and the ticks between hits.
local KINDS = {
    [0] = { palette = 1, rearm = 4 },
    { palette = 0, rearm = 4 },
    { palette = 3, rearm = 4 },
    { palette = 3, rearm = 4 },
    { palette = 4, rearm = 4 },
    { palette = 2, rearm = 4 },
    { palette = 5, rearm = 4 },
    { palette = 0, rearm = 0 },
}
-- What a plain tornado becomes on a panel of these types
-- (`byte_80CA25C`).
local PANEL_KINDS: { [PanelType]: number } = { volcano = 4, grass = 0, ice = 6 }
local PLAIN = 1
-- Kind 3's HP bug: bug 0x54, argument 5 (`sub_801A4D0`).
local HP_BUG = 0x54 + 5 * 0x100
-- The damage bits that are flags, kept when the damage doubles.
local DAMAGE_FLAGS = 0xF800

-- `sub_80CA0E2`: a tornado for `owner`'s side on (x, y) with the damage
-- word `damage` and `params` (kind, status, and its ticks in the third and
-- fourth). Its position is the spawner's registers (the panel Y, the
-- element and 0) until its init places it.
function tornado.spawn(owner: Object, x: number, y: number, element: number, damage: number, params: { number }): Object?
    local o = battle.spawn_kind("tornado", Vec3.new(y, element, 0), params)
    if o then
        o.element = element
        o.panel_x, o.panel_y = x, y
        o.timer = o:param(3) + o:param(4) * 0x100
        o.damage = damage
        o.stamina = bit32.rshift(damage, 16)
        o.alliance = owner.alliance
    end
    return o
end

-- The destroy state, hidden and harmless.
local function finish(me: Object)
    me.collision.region = 0
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
    me.collision.region = 1
end

-- `sub_80CA074`: resolve its hits (a hit takes its region away), spin.
local function tick(me: Object)
    local c = me.collision
    c:remove()
    me:hit_spark()
    if battle.time_up() then
        me.visible = false
        c.region = 0
        me:set_lifecycle("destroy")
        return
    end
    if c.hit_flags ~= 0 then
        c.region = 0
    end
    spin(me)
    me:update_sprite()
    c:present()
end

-- `sub_80CA234`: the kind a plain tornado takes from its panel (1 when it
-- stays plain).
local function kind_on_panel(me: Object): number
    local p = field.panel(me.panel_x, me.panel_y)
    -- (Off the field the game reads a type from address 0 and the tornado
    -- frees itself right after, whatever it found.)
    return p and PANEL_KINDS[p.kind] or PLAIN
end

-- `sub_80C9F98`: take the panel's element, then spin on it.
local function init(me: Object)
    if me:param(1) == PLAIN then
        local kind = kind_on_panel(me)
        me:set_param(1, kind)
        if kind ~= PLAIN then
            field.set_type(me.panel_x, me.panel_y, "normal")
            local d = me.damage
            me.damage = bit32.bor(bit32.band(d, DAMAGE_FLAGS), bit32.band(d, bit32.bnot(DAMAGE_FLAGS)) * 2)
        end
    end
    if not field.valid(me.panel_x, me.panel_y) then
        me:free()
        return
    end
    me.visible = true
    local sprite = me.sprite
    sprite:load(SPRITE)
    sprite.shadow = "with_sprite"
    me.anim = 0
    me.anim_loaded = 0
    sprite:set_animation(0)
    sprite:set_flip(bit32.bxor(me.alliance, me.flip))
    local s = me.state :: State
    s.rearm_step = 1
    me:set_coordinates_from_panel()
    local p = me.pos
    me.pos = Vec3.new(p.x, p.y, 0)
    local kind = me:param(1)
    local k = KINDS[kind]
    if k == nil then
        error(string.format("tornado kind %d reads past byte_80CA064", kind))
    end
    s.rearm_ticks = k.rearm
    me.timer2 = k.rearm
    sprite.palette = k.palette
    if not me:create_collision() then
        me:free()
        return
    end
    me:setup_collision(0x12, 5, 1)
    local c = me.collision
    c.hit_effect = bit32.band(me.element, 0xF)
    local status = me:param(2)
    if status ~= 0 then
        c.status_base = status
    end
    if kind == 3 then
        c.bugs = HP_BUG
    end
    c:present()
    battle.play_sound(0xB8)
    me:set_lifecycle("update")
    tick(me)
end

-- `sub_80C9F78`.
function tornado.update(me: Object)
    local l = me.lifecycle
    if l == "init" then
        init(me)
    elseif l == "update" then
        tick(me)
    else
        me:destroy()
    end
end

return tornado
