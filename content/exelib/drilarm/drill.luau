--!strict
-- A drill, as EXE5 and EXE6 share it (EXE6's attack object #0x71,
-- `sub_80D2A94`; EXE5's #0x71): DrilArm's
-- (./chip), which GroundCross's charged shot
-- (navis/megaman/forms/groundcross/drill) and MstrCros's GroundCross
-- (chips/mstrcros/navi) use too. It sits on the panel ahead of its owner
-- (following it), invisible, hitting what stands there every 12 ticks for
-- its time; a hit takes its region away until the next 12. The first one
-- brings a second drill on the panel ahead of itself. Each goes as soon as
-- the slot that holds it lets go of it: its navi's first related object,
-- or, for the second, the first drill's link, which the first clears when
-- it goes. A hit also shakes the camera. Each game's drill is a kind of its
-- look (`drill.make_kind`: its spark, its collision and what its spawn
-- keeps; EXE5's DrilArm and DarkDril). Its third parameter is its user's:
-- EXE6's makes a light one, EXE5's a dark one (`Spec`).

local regions = require("../regions")

local drill = {}

export type State = {
    -- Where it is held (ExtraVars+0, a pointer in the game): its owner's
    -- first related object, or its owner's second drill.
    slot: "related" | "child",
    -- The second drill (ExtraVars+8).
    child: Object?,
    -- It is the second (Param1 1), its ticks of hitting (Param2), and
    -- its Param3 (see `Spec`).
    second: boolean,
    ticks: number,
    light: boolean,
    bugs: number,
}

-- How long a drill hits, and what its Param3 makes it, its user's: EXE6's
-- light one (`light`), which hits once, with hit modifier 3 (MstrCros's
-- GroundCross); EXE5's dark one, whose hit carries a bug (`bugs`, the
-- collision's bug word, the bug in its low byte and its argument above:
-- DarkDril's; EXE5's 0x080D4154, `sub_801A4D0`).
export type Spec = { ticks: number, light: boolean?, bugs: number? }


-- Ticks between hits.
local PERIOD = 0x0C
-- Hit modifiers.
local HIT_MOD, LIGHT_HIT_MOD = 0x61, 3

-- A drill kind (`id`) of `look`, with its spawner.
function drill.make_kind(
    id: string,
    look: DrillLook
): (Kind, (owner: Object, element: number, z: number, damage: number, spec: Spec, slot: ("related" | "child")?) -> Object?)
    local kind: Kind

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
        local o = battle.spawn(kind, Vec3.new(0, element, z))
        if o then
            if look.keeps_element then
                o.element = element
            end
            o.alliance, o.flip = owner.alliance, owner.flip
            o.related1 = owner
            o:set_damage_word(damage)
            local s = o.state :: State
            s.second, s.ticks, s.light, s.bugs = second, spec.ticks, spec.light or false, spec.bugs or 0
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
            local o = spawn(me, me.element, me.pos.z, damage, { ticks = s.ticks, light = s.light, bugs = s.bugs }, true, "child")
            if o and look.keeps_element then
                o.run_while_dimmed = me.run_while_dimmed
            end
        end
        if not me:create_collision() then
            s.child = nil
            me:free()
            return
        end
        local hit_mod = if s.light then LIGHT_HIT_MOD else HIT_MOD
        me:setup_collision(if s.second then look.second else look.first, look.hits, hit_mod)
        me.timer2 = s.ticks
        me.timer = PERIOD
        follow(me)
        local c = me.collision
        c:set_region(regions.single)
        c:set_hit_effect(look.spark)
        if s.bugs ~= 0 then
            -- (EXE5's 0x080D4154: sub_801A4D0.)
            c.bugs = s.bugs
        end
        me:set_lifecycle("update")
        tick(me)
    end

    kind = new.kind {
        id = id,
        pool = "attack",
        state = {
            slot = { "related", "child" },
            child = "object",
            second = "bool",
            ticks = "u8",
            light = "bool",
            bugs = "u16",
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

    return kind, function(owner: Object, element: number, z: number, damage: number, spec: Spec, slot: ("related" | "child")?): Object?
        return spawn(owner, element, z, damage, spec, false, slot or "related")
    end
end

return drill
