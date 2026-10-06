--!strict
-- A drip shower (the original's attack object #0x29, `sub_80C91A0`):
-- SpoutMan's DripShwr. It stands on its panel and sprays a pair of the
-- panels around it, turning a quarter every few ticks: a hit on each for
-- as long, and the next pair highlighted. It stands still while dimmed,
-- and goes when the battle is over (or, one that lasts through its owner's
-- action, when he leaves it). See docs/engine/standard-chips.md, "Action
-- 0x0A".

local collision = require("../../rules/collision")
local regions = require("@exelib/regions")
local sparks = require("../../lib/sparks")

local drip_shower = {}

-- What it is spawned with: its turns (Param2) and the ticks a quarter
-- lasts (Param3); and whether it lasts only while its owner stays in his
-- action (Param1 other than DripShwr's 4).
type Spec = DripShower

export type State = {
    turns: number,
    quarter_ticks: number,
    -- The action it lasts through (none: it lasts its time).
    action: Action?,
    -- The quarter it sprays (ExtraVars+0).
    quarter: number,
    -- Ticks to its next splash sound (ExtraVars+4).
    sound: number,
}

local SPRITE = asset.sprite("drip-shower")
local SOUND = asset.sound("drip-shower")
local SOUND_EVERY = 0x10

-- Its hits: aqua spark, an attack at navis.
local HIT_MOD = 3

-- The two panels of each quarter (`byte_80C93B0`), not turned to its side;
-- the quarter each turn sprays, by side (`byte_80C93C4`); and its
-- animation, by side and turn (`byte_80C92C4`).
local QUARTERS = {
    [0] = { { -1, 0 }, { 1, 0 } },
    { { -1, -1 }, { 1, 1 } },
    { { 0, -1 }, { 0, 1 } },
    { { 1, -1 }, { -1, 1 } },
}
local ORDER = { [0] = { [0] = 0, 3, 2, 1 }, [1] = { [0] = 0, 1, 2, 3 } }
local ANIMS = { [0] = { [0] = 0, 1, 2, 3 }, [1] = { [0] = 0, 3, 2, 1 } }

local function quarter_panels(me: Object, turn: number): { { number } }
    local q = QUARTERS[ORDER[me.alliance][turn]]
    if q == nil then
        error(string.format("drip shower quarter %d reads past byte_80C93C4", turn))
    end
    return q
end

-- `sub_80C9360`: highlight the quarter's panels.
local function highlight(me: Object, turn: number)
    for _, d in quarter_panels(me, turn) do
        local x, y = me.panel_x + d[1], me.panel_y + d[2]
        if field.valid(x, y) then
            field.highlight(x, y)
        end
    end
end

-- `sub_80C92F4`: highlight the quarter's panels and hit them for `ticks`.
local function splash(me: Object, turn: number, ticks: number)
    for _, d in quarter_panels(me, turn) do
        local x, y = me.panel_x + d[1], me.panel_y + d[2]
        if field.valid(x, y) then
            field.highlight(x, y)
            local hit = battle.hitbox(me, {
                panel_x = x,
                panel_y = y,
                element = me.element,
                z = 0,
                region = regions.single,
                hit_effect = sparks.aqua,
                target = collision.hits_navis,
                self_type = collision.attack,
                damage = me.damage,
                stamina = me.stamina,
                hit_mod = HIT_MOD,
            })
            if hit then
                hit.timer = ticks
            end
        end
    end
end

-- `sub_80C9238`: spray, a quarter every few ticks, for its turns.
local function spray(me: Object)
    local s = me.state :: State
    local ticks = s.quarter_ticks
    if me.phase == 0 then
        me.phase = 4
        s.quarter = 0
        me.timer = ticks * 4 * s.turns + ticks
        me.timer2 = ticks
        splash(me, 0, ticks)
        s.sound = 0
    end
    local sound = s.sound - 1
    s.sound = sound
    if sound <= 0 then
        battle.play_sound(SOUND)
        s.sound = SOUND_EVERY
    end
    local t = me.timer - 1
    me.timer = t
    if t <= 0 then
        me:set_lifecycle("destroy")
        return
    end
    highlight(me, s.quarter)
    local left = me.timer2 - 1
    me.timer2 = left
    if left > 0 then
        return
    end
    local q = (s.quarter + 1) % 4
    s.quarter = q
    me.anim = ANIMS[me.alliance][q]
    me.timer2 = ticks
    splash(me, q, ticks)
end

-- `sub_80C91FE`: gone when the battle is over (or its owner left his
-- action); still while dimmed.
local function tick(me: Object)
    local s = me.state :: State
    if battle.over() or (s.action ~= nil and (me.related1 :: Object):navi_action() ~= s.action) then
        me:set_lifecycle("destroy")
        return
    end
    if battle.dimmed() then
        return
    end
    if me.action ~= 0 then
        error(string.format("drip shower action %#x reads past off_80C9234", me.action))
    end
    spray(me)
end

-- `sub_80C91C4`: on its panel, on the ground (keeping its Z's fraction).
local function init(me: Object)
    me:set_coordinates_from_panel()
    local p = me.pos
    me.pos = Vec3.new(p.x, p.y, bit32.band(p.z, 0xFFFF))
    local sprite = me.sprite
    sprite:load(SPRITE)
    sprite.shadow = "with_sprite"
    me.visible = true
    me.anim = 0
    me.anim_loaded = 0
    sprite:set_animation(0)
    sprite.palette = 0
    me:set_lifecycle("update")
end

drip_shower.kind = new.kind {
    id = "spoutman/drip-shower",
    pool = "attack",
    state = { turns = "u8", quarter_ticks = "u8", action = "action", quarter = "u32", sound = "i32" },
    -- `sub_80C91A0`.
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
        me:update_sprite_while_paused()
    end,
}

-- `sub_80C92CC`: a drip shower for `owner` on (x, y), with `element` and
-- the damage word `damage`. It runs while dimmed (and stands still).
-- `pos` is where its spawner's registers leave it (its row, the element
-- and a Z), of which its init keeps the Z's fraction.
function drip_shower.spawn(owner: Object, x: number, y: number, element: number, spec: Spec, damage: number, pos: Vec3): Object?
    local o = battle.spawn(drip_shower.kind, pos)
    if o then
        o.panel_x, o.panel_y = x, y
        o.element = element
        o:set_damage_word(damage)
        o.related1 = owner
        o.alliance, o.flip = owner.alliance, owner.flip
        o.run_while_dimmed = true
        local s = o.state :: State
        s.turns, s.quarter_ticks = spec.turns, spec.quarter_ticks
        if spec.follows_action then
            s.action = owner:navi_action() :: Action
        end
    end
    return o
end

return drip_shower
