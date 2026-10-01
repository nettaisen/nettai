--!strict
-- A drill (the original's attack object #0x71, `sub_80D2A94`): DrilArm's
-- (./chip), which GroundCross's charged shot
-- (navis/megaman/forms/groundcross/drill) and MstrCros's GroundCross
-- (chips/mstrcros/navi) use too. It sits on the panel ahead of its owner
-- (following it), invisible, hitting what stands there every 12 ticks for
-- its time; a hit takes its region away until the next 12. The first one
-- brings a second drill on the panel ahead of itself. Each goes as soon as
-- the slot that holds it lets go of it: its navi's first related object,
-- or, for the second, the first drill's link, which the first clears when
-- it goes. A hit also shakes the camera.

local attachment = require("../../objects/attachment/attachment")
local collision = require("../../rules/collision")
local regions = require("../../lib/regions")
local sparks = require("../../lib/sparks")

local drill = {}

-- The drill arm the navi wears while it drills (the original's attachment
-- row 0x20), in its drilling animation (1) or drawn back (0).
drill.arm = attachment.look { sprite = asset.sprite("drill-arm"), palette = 0, attach_point = 0x13 }
drill.ARM_DRILLING = 1

export type State = {
    -- Where it is held (ExtraVars+0, a pointer in the game): its owner's
    -- first related object, or its owner's second drill.
    slot: "related" | "child",
    -- The second drill (ExtraVars+8).
    child: Object?,
    -- It is the second (Param1 1), its ticks of hitting (Param2), and a
    -- light one that hits once, with hit modifier 3 (Param3 1).
    second: boolean,
    ticks: number,
    light: boolean,
}

-- How long a drill hits, and whether lightly (`Param2`, `Param3`).
export type Spec = { ticks: number, light: boolean? }

-- Ticks between hits.
local PERIOD = 0x0C
-- Hit modifiers.
local HIT_MOD, LIGHT_HIT_MOD = 0x61, 3

-- Whether the slot that holds it still does.
local function held(me: Object): boolean
    local s = me.state :: State
    local owner = me.related1 :: Object
    if s.slot == "related" then
        return owner.related1 == me
    end
    return (owner.state :: State).child == me
end

-- `sub_80D2BC0`: onto the panel ahead of its owner.
local function follow(me: Object)
    local owner = me.related1 :: Object
    me.panel_x = owner.panel_x + me.facing
    me.panel_y = owner.panel_y
    me:set_coordinates_from_panel()
    me:update_collision_panels()
end

-- `sub_80D2B2C`: resolve its hits; count down; follow.
local function tick(me: Object)
    local c = me.collision
    local s = me.state :: State
    c:remove()
    me:hit_spark()
    local done = not held(me)
    if not done then
        if c.hit_flags ~= 0 then
            c:set_region(nil)
            battle.shake_camera(0, 0xC)
        end
        local left = me.timer2 - 1
        me.timer2 = left
        if left < 0 then
            done = true
        else
            local t = me.timer - 1
            me.timer = t
            if t <= 0 then
                me.timer = PERIOD
                if not s.light then
                    c:set_region(regions.single)
                end
            end
        end
    end
    if done then
        c:set_region(nil)
        s.child = nil
        me:set_lifecycle("destroy")
    end
    follow(me)
    c:present()
end

-- `sub_80D2B8E`: a drill for `owner` with the damage word `damage`, at
-- height `z` (the rest of its position is the spawner's registers until
-- its init places it), held in `slot` of `owner`.
local function spawn(
    owner: Object,
    element: number,
    z: number,
    damage: number,
    spec: Spec,
    second: boolean,
    slot: "related" | "child"
): Object?
    local o = battle.spawn(drill.kind, Vec3.new(0, element, z))
    if o then
        o.element = element
        o.alliance, o.flip = owner.alliance, owner.flip
        o.related1 = owner
        o:set_damage_word(damage)
        local s = o.state :: State
        s.second, s.ticks, s.light = second, spec.ticks, spec.light or false
        s.slot = slot
        if slot == "related" then
            owner.related1 = o
        else
            (owner.state :: State).child = o
        end
    end
    return o
end

-- `sub_80D2AB4`: the first brings the second; then on its panel.
local function init(me: Object)
    local s = me.state :: State
    if not s.second then
        local damage = me.damage + me.stamina * 0x10000
        local o = spawn(me, me.element, me.pos.z, damage, { ticks = s.ticks, light = s.light }, true, "child")
        if o then
            o.run_while_dimmed = me.run_while_dimmed
        end
    end
    if not me:create_collision() then
        s.child = nil
        me:free()
        return
    end
    local hit_mod = if s.light then LIGHT_HIT_MOD else HIT_MOD
    me:setup_collision(if s.second then collision.second_drill else collision.breaking, collision.hits_navis, hit_mod)
    me.timer2 = s.ticks
    me.timer = PERIOD
    follow(me)
    local c = me.collision
    c:set_region(regions.single)
    c:set_hit_effect(sparks.breaking)
    me:set_lifecycle("update")
    tick(me)
end

drill.kind = define.kind {
    id = "drilarm/drill",
    pool = "attack",
    state = {
        slot = { "related", "child" },
        child = "object",
        second = "bool",
        ticks = "u8",
        light = "bool",
    },
    -- `sub_80D2A94`.
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

-- `sub_80D2B8E` from its user: the first drill for `owner`, held in its
-- first related object (a navi's attack), or in its state's `child` (a
-- navi chip's navi, whose kind has that field).
function drill.spawn(
    owner: Object,
    element: number,
    z: number,
    damage: number,
    spec: Spec,
    slot: ("related" | "child")?
): Object?
    return spawn(owner, element, z, damage, spec, false, slot or "related")
end

return drill
