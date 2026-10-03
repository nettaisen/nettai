--!strict
-- A riding hit (the original's attack object #0x69, `sub_80D18D8`): an
-- unseen hit region on its owner, following him panel to panel while he
-- stays in the action that put it on him (SlashMan's RSlash rolling). A hit
-- on a body ends it, unless it keeps going: then its region is off until
-- it reaches another panel. See docs/engine/standard-chips.md, "Action
-- 0x0A".

local regions = require("../../lib/regions")

local riding_hit = {}

-- What it is spawned with: its collision types (the original's ExtraVars
-- +0 and +4), hit modifier and spark (Param1, Param2), and whether it goes
-- on after hitting a body (Param3).
type Spec = RidingHit

export type State = {
    is: CollisionType,
    hits: CollisionType,
    hit_mod: number,
    spark: Spark?,
    keeps_going: boolean,
    -- The action its owner was in when it came (ExtraVars+8).
    action: Action?,
}

-- What it counts as a body, by side (`byte_80D19B8`: the other side's
-- bodies and objects, and neutral objects).
local BODIES = { [0] = 0x0580_0000, 0x0A80_0000 }

-- Its owner's position, and the panel under it.
local function follow(me: Object)
    me.pos = (me.related1 :: Object).pos
    me:set_panel_from_coordinates()
end

-- Ending: its region cleared, then the destroy state.
local function finish(me: Object)
    me.collision:set_region(nil)
    me:set_lifecycle("destroy")
end

-- `sub_80D18F8`: on its owner, collision on.
local function init(me: Object)
    follow(me)
    if not me:create_collision() then
        me:free()
        return
    end
    local s = me.state :: State
    me:setup_collision(s.is, s.hits, s.hit_mod)
    me.collision:set_hit_effect(s.spark)
    me.collision:present()
    me:set_lifecycle("update")
end

-- `sub_80D1934`: resolve its hits; follow the owner while he stays in his
-- action.
local function tick(me: Object)
    local c = me.collision
    c:remove()
    me:hit_spark()
    local s = me.state :: State
    if battle.over() or (me.related1 :: Object):navi_action() ~= s.action then
        return finish(me)
    end
    if bit32.btest(c.hit_flags, BODIES[me.alliance]) then
        if not s.keeps_going then
            return finish(me)
        end
        c:set_region(nil)
    end
    local x, y = me.panel_x, me.panel_y
    if not field.valid(x, y) then
        return finish(me)
    end
    follow(me)
    me:update_collision_panels()
    if me.panel_x ~= x or me.panel_y ~= y then
        c:set_region(regions.single)
    end
    c:present()
end

-- `sub_80D19C0`: resolve its last hits, then go.
local function destroy(me: Object)
    local c = me.collision
    c:remove()
    me:hit_spark()
    c:set_region(nil)
    me:destroy()
end

riding_hit.kind = define.kind {
    id = "bn6:slashman/riding-hit",
    pool = "attack",
    state = { is = "collision", hits = "collision", hit_mod = "u8", spark = "spark", keeps_going = "bool", action = "action" },
    -- `sub_80D18D8`.
    update = function(me: Object)
        local l = me.lifecycle
        if l == "init" then
            init(me)
        elseif l == "update" then
            tick(me)
        else
            destroy(me)
        end
    end,
}

-- `sub_80D19D4`: a riding hit for `owner`, in the action he is running,
-- with `element` and the damage word `damage`. It runs while dimmed; its
-- position is its owner's once its init runs.
function riding_hit.spawn(owner: Object, element: number, spec: Spec, damage: number): Object?
    local o = battle.spawn(riding_hit.kind, Vec3.zero)
    if o then
        local s = o.state :: State
        s.is, s.hits, s.hit_mod, s.spark, s.keeps_going = spec.is, spec.hits, spec.hit_mod, spec.spark, spec.keeps_going
        s.action = owner:navi_action() :: Action
        o.element = element
        o:set_damage_word(damage)
        o.alliance, o.flip = owner.alliance, owner.flip
        o.related1 = owner
        o.run_while_dimmed = true
    end
    return o
end

-- `sub_80D1A00`: end it (its next update frees it).
function riding_hit.stop(o: Object)
    o:set_lifecycle("destroy")
end

return riding_hit
