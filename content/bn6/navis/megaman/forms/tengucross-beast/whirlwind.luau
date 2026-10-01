--!strict
-- A whirlwind (the original's attack object #0x81, `sub_80D5580`),
-- TenguCross Beast's charged chip's (./charge): it stands 40 pixels ahead
-- of its panel for its ticks, highlighting the panels it sweeps, and
-- leaves lingering hit regions on its way: as it passes 45, 30 and 15
-- ticks left (`off_80D56C4`'s three waves), or all at once on its first
-- tick. Once its owner leaves the action that sent it, it ends 4 ticks
-- later.

local collision = require("../../../../rules/collision")
local regions = require("../../../../lib/regions")
local sparks = require("../../../../lib/sparks")

local whirlwind = {}

-- What a whirlwind is spawned with (the original's parameters): whether
-- its hits come all at once (Param1), how long it stands (Param3) and its
-- animation (Param4). (Param2 named its owner's action by number.)
export type Spec = { at_once: boolean, ticks: number, anim: number }

export type State = {
    at_once: boolean,
    ticks: number,
    anim: number,
    -- The action its owner sent it in.
    action: Action?,
}

local SPRITE = asset.sprite("whirlwind")

-- How far ahead of its panel's center it stands (`byte_80D5738`), 16.16.
local AHEAD = 0x28_0000

-- The height of its hits, 16.16, and their hit modifier.
local HIT_Z, HIT_MOD = 0x10_0000, 1

-- The last panels it sweeps: the column two panels ahead.
local SWEEP = regions.wide

-- A wave's hits (`byte_80D56D0`, `byte_80D56E8`, `byte_80D5700`): each
-- entry's columns ahead, how long it lingers, and the hit's region: a
-- pushing attack at navis, with the plain spark. `false`: none.
type Hit = { ahead: number, ticks: number, region: Region }

local function hit(ahead: number, ticks: number, region: Region): Hit
    return { ahead = ahead, ticks = ticks, region = region }
end

local WAVES: { [number]: { Hit | false } } = {
    [0] = { hit(0, 0x2D, regions.single), hit(1, 0x0F, regions.single), hit(2, 0x0F, regions.wide) },
    [1] = { false, hit(1, 0x1E, regions.single), hit(2, 0x0F, regions.wide) },
    [2] = { false, false, hit(2, 0x0F, regions.wide) },
}

-- The wave by the ticks left, when they come in turn.
local WAVE_TICKS = { [0x2D] = 0, [0x1E] = 1, [0x0F] = 2 }

-- `sub_80D5684`: one of a wave's hits, lingering its ticks.
local function strike(me: Object, h: Hit)
    local box = battle.hitbox(me, {
        panel_x = me.panel_x + me.facing * h.ahead,
        panel_y = me.panel_y,
        element = me.element,
        z = HIT_Z,
        region = h.region,
        hit_effect = sparks.plain,
        target = collision.hits_navis,
        self_type = collision.pushing,
        damage = me.damage,
        stamina = me.stamina,
        hit_mod = HIT_MOD,
    })
    if box then
        box.timer = h.ticks
    end
end

-- `sub_80D5634`: 4 more ticks once the owner leaves its action; a wave of
-- hits at its ticks.
local function waves(me: Object, s: State)
    if me.action == 0 and (me.related1 :: Object):navi_action() ~= s.action then
        me.timer = 4
        me.action = 4
    end
    local wave: number? = 0
    if s.at_once then
        if me.timer ~= s.ticks then
            return
        end
    else
        wave = WAVE_TICKS[me.timer]
        if wave == nil then
            return
        end
    end
    for _, h in WAVES[wave :: number] do
        if h then
            strike(me, h)
        end
    end
end

-- `sub_8109660(x, y, region, 0)`: highlight a region's panels on the field
-- around (x, y), facing right whatever the object's side.
local function highlight_region(x: number, y: number, region: Region)
    for _, offset in region.panels :: { { number } } do
        local px, py = x + offset[1], y + offset[2]
        if px >= 1 and px <= 6 and py >= 1 and py <= 3 then
            field.highlight(px, py)
        end
    end
end

-- `sub_80D55F8`: the waves; then highlight the panels it sweeps, or end.
local function tick(me: Object)
    waves(me, me.state :: State)
    local t = me.timer - 1
    me.timer = t
    if t <= 0 then
        me:set_lifecycle("destroy")
        return
    end
    local front = me.facing
    local x, y = me.panel_x, me.panel_y
    field.highlight(x, y)
    field.highlight(x + front, y)
    highlight_region(x + 2 * front, y, SWEEP)
end

-- `sub_80D55A4`: ahead of its panel, in its animation.
local function init(me: Object)
    local s = me.state :: State
    local sprite = me.sprite
    sprite:load(SPRITE)
    sprite.shadow = "hidden"
    me.visible = true
    local anim = s.anim
    me.anim = anim
    me.anim_loaded = anim
    sprite:set_animation(anim)
    sprite:set_flip(bit32.bxor(me.alliance, me.flip))
    sprite.palette = 0
    me.timer = s.ticks
    me:set_coordinates_from_panel()
    local p = me.pos
    me.pos = Vec3.new(p.x + me.facing * AHEAD, p.y, p.z)
    me:set_lifecycle("update")
    tick(me)
end

whirlwind.kind = define.kind {
    id = "tengucross-beast/whirlwind",
    pool = "attack",
    state = { at_once = "bool", ticks = "u8", anim = "u8", action = "action" },
    -- `sub_80D5580`.
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

-- `sub_80D5718`: a whirlwind from `owner` on (x, y), lasting while he
-- stays in the action he is running, with `element` and the damage word.
-- Its position is the registers (y, the element, 0) until its init places
-- it.
function whirlwind.spawn(owner: Object, x: number, y: number, element: number, spec: Spec, damage: number): Object?
    local o = battle.spawn(whirlwind.kind, Vec3.new(y, element, 0))
    if o then
        local s = o.state :: State
        s.at_once, s.ticks, s.anim = spec.at_once, spec.ticks, spec.anim
        local action = owner:navi_action()
        if type(action) ~= "table" then
            error("a whirlwind follows an action that is no content action")
        end
        s.action = action :: Action
        o.panel_x, o.panel_y = x, y
        o.element = element
        o.alliance, o.flip = owner.alliance, owner.flip
        o:set_damage_word(damage)
        o.related1 = owner
    end
    return o
end

return whirlwind
