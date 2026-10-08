--!strict
-- A dash's hit (the original's attack object #0xAF, `sub_80DB570`): an
-- invisible hit region that rides along with its owner, some steps of 58
-- pixels in front of it, while the owner stays in the action that sent it:
-- GroundCross Beast's dash (forms/groundcross-beast/dash: two of them) and
-- ChargeCross's charged tackle (forms/chargecross/tackle), each its own
-- variant (`dash_hit.variant`; the original's second parameter picks it).
-- A tackle's ends after its first hit and reports that it is there in its
-- owner's marker word (the spawner passes the address of the attack's
-- +0x30); a dash's lets go of whatever it hit and hits again on the next
-- panel, but ends on reaching it (the region it restores leaves 1 in the
-- register compared with the owner's action).

local collision = require("../../rules/collision")
local regions = require("@exelib/regions")

local dash_hit = {}

-- A user's hit: its hit modifier and spark, and whether it reports in its
-- owner's marker word, ending after its first hit (the tackle's: the
-- original's second parameter, the marker's address; the dash's 0).
export type Variant = { hit_mod: number, spark: Spark, reports: boolean }

function dash_hit.variant(v: Variant): Variant
    return new.record("megaman-dash-hit-variant", v)
end

export type State = {
    -- ExtraVars: the panel it last hit on (PanelXY: x | y << 8).
    hit_panel: number,
    -- How many steps in front of its owner it rides (the original's first
    -- parameter), and its user's variant.
    reach: number,
    variant: Variant,
    -- The action its owner sent it in.
    action: Action?,
}

-- How far in front of its owner it rides, per step of its reach, 16.16.
local REACH = 0x3A_0000

-- `sub_80DB684`: in front of the owner, on its row.
local function follow(me: Object)
    local owner = me.related1 :: Object
    local p = me.pos
    local x = owner.pos.x + me.facing * (me.state :: State).reach * REACH
    me.pos = Vec3.new(x, owner.pos.y, p.z)
end

-- Gone: a reporting one says so in its owner's marker word.
local function finish(me: Object)
    if (me.state :: State).variant.reports then
        (me.related1 :: Object).marker = 0
    end
    local c = me.collision
    c:set_region(nil)
    c:free()
    me:free()
end

-- `sub_80DB5E8`.
local function tick(me: Object)
    local c = me.collision
    c:remove()
    me:hit_spark()
    local s = me.state :: State
    -- It ends once its owner has left the action. After its region comes
    -- back, the register the game compares with the owner's action holds
    -- the region instead: 1, the number of the state a navi takes control
    -- in, which no navi it rides is in, so it ends then too.
    local running = (me.related1 :: Object):navi_action()
    local gone = running ~= s.action
    if c.region == nil and s.hit_panel ~= me.panel_x + me.panel_y * 0x100 then
        c:set_region(regions.single)
        gone = running ~= "take_control"
    end
    if gone then
        return finish(me)
    end
    if c.hit_flags ~= 0 then
        c:set_region(nil)
        s.hit_panel = me.panel_x + me.panel_y * 0x100
        if s.variant.reports then
            return finish(me)
        end
    end
    follow(me)
    me:set_panel_from_coordinates()
    me:update_collision_panels()
    if not field.valid(me.panel_x, me.panel_y) then
        return finish(me)
    end
    c:present()
end

-- `sub_80DB590`: its collision (its variant's hit modifier and spark),
-- placed on its panel (still 0, 0) and then updated at once. A reporting
-- one reports.
local function init(me: Object)
    if not me:create_collision() then
        me:free()
        return
    end
    local v = (me.state :: State).variant
    me:setup_collision(collision.breaking, collision.hits_navis, v.hit_mod)
    me.collision:set_hit_effect(v.spark)
    follow(me)
    me:set_coordinates_from_panel()
    me:update_collision_panels()
    me.collision:present()
    if v.reports then
        (me.related1 :: Object).marker = 1
    end
    me:set_lifecycle("update")
    tick(me)
end

dash_hit.kind = new.kind {
    id = "megaman/dash-hit",
    pool = "attack",
    state = { hit_panel = "u16", reach = "u8", variant = "record:megaman-dash-hit-variant", action = "action" },
    -- `sub_80DB570`.
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

-- `sub_80DB666`: a hit for `owner`, riding while he stays in the action he
-- is running, with `element` and the damage word `damage`, `reach` steps
-- in front of it, of `variant`. Its Y is the element (the spawner's
-- registers) and its X register garbage, until its init places it; Z is
-- `z`.
function dash_hit.spawn(owner: Object, element: number, damage: number, reach: number, variant: Variant, z: number): Object?
    local o = battle.spawn(dash_hit.kind, Vec3.new(0, element, z))
    if o then
        local s = o.state :: State
        s.reach, s.variant = reach, variant
        local action = owner:navi_action()
        if type(action) ~= "table" then
            error("a dash's hit follows an action that is no content action")
        end
        s.action = action :: Action
        o.element = element
        o:set_damage_word(damage)
        o.alliance, o.flip = owner.alliance, owner.flip
        o.related1 = owner
    end
    return o
end

return dash_hit
