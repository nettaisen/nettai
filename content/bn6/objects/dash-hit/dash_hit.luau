--!strict
-- A dash's hit (attack object #0xAF, `sub_80DB570`): an invisible hit
-- region that rides along with its owner, Param1 times 58 pixels in front
-- of it, while the owner stays in the dash's action: GroundCross Beast's
-- dash (action 0x1A, two of them, Param2 0) and ChargeCross's charged
-- tackle (action 0x56, Param2 set). A tackle's ends after its first hit and
-- reports that it is there in its owner's marker word (the spawner passes
-- the address of the attack's +0x30); a dash's lets go of whatever it hit
-- and hits again on the next panel, but ends on reaching it (the region it
-- restores leaves 1 in the register compared with the owner's action).

local dash_hit = {}

export type State = {
    -- ExtraVars: the panel it last hit on (PanelXY: x | y << 8).
    hit_panel: number,
}

dash_hit.state = { hit_panel = "u16" }

-- How far in front of its owner it rides, per step of Param1, 16.16.
local REACH = 0x3A_0000

-- The owner's actions it rides along: the dash's, and the tackle's.
local DASH_ACTION, TACKLE_ACTION = 0x1A, 0x56

-- `sub_80DB666`: a hit for `owner` with `element` and the damage word
-- `damage`, `reach` (Param1) steps in front of it; `tackle` (Param2) for
-- the charged tackle's. Its Y is the element (the spawner's registers)
-- and its X register garbage, until its init places it; Z is `z`.
function dash_hit.spawn(owner: Object, element: number, damage: number, reach: number, tackle: boolean, z: number): Object?
    local o = battle.spawn_kind("dash-hit", Vec3.new(0, element, z), { reach, if tackle then 1 else 0 })
    if o then
        o.element = element
        o.damage = damage
        o.stamina = bit32.rshift(damage, 16)
        o.alliance, o.flip = owner.alliance, owner.flip
        o.related1 = owner
    end
    return o
end

-- `sub_80DB684`: in front of the owner, on its row.
local function follow(me: Object)
    local owner = me.related1 :: Object
    local p = me.pos
    local x = owner.pos.x + me.facing * me:param(1) * REACH
    me.pos = Vec3.new(x, owner.pos.y, p.z)
end

-- Gone: a tackle's says so in its owner's marker word.
local function finish(me: Object)
    if me:param(2) ~= 0 then
        (me.related1 :: Object).marker = 0
    end
    local c = me.collision
    c.region = 0
    c:free()
    me:free()
end

-- `sub_80DB5E8`.
local function tick(me: Object)
    local c = me.collision
    c:remove()
    me:hit_spark()
    local s = me.state :: State
    local tackle = me:param(2) ~= 0
    -- The owner's action it expects; after its region comes back, the
    -- register holds the region instead.
    local expected
    if c.region == 0 and s.hit_panel ~= me.panel_x + me.panel_y * 0x100 then
        c.region = 1
        expected = 1
    else
        expected = if tackle then TACKLE_ACTION else DASH_ACTION
    end
    if (me.related1 :: Object).action ~= expected then
        return finish(me)
    end
    if c.hit_flags ~= 0 then
        c.region = 0
        s.hit_panel = me.panel_x + me.panel_y * 0x100
        if tackle then
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

-- `sub_80DB590`: its collision (a tackle's harder, with a spark), placed
-- on its panel (still 0, 0) and then updated at once. A tackle's reports.
local function init(me: Object)
    if not me:create_collision() then
        me:free()
        return
    end
    local tackle = me:param(2) ~= 0
    me:setup_collision(6, 5, if tackle then 3 else 1)
    me.collision.hit_effect = if tackle then 1 else 0
    follow(me)
    me:set_coordinates_from_panel()
    me:update_collision_panels()
    me.collision:present()
    if tackle then
        (me.related1 :: Object).marker = 1
    end
    me:set_lifecycle("update")
    tick(me)
end

-- `sub_80DB570`.
function dash_hit.update(me: Object)
    local l = me.lifecycle
    if l == "init" then
        init(me)
    elseif l == "update" then
        tick(me)
    else
        me:destroy()
    end
end

return dash_hit
