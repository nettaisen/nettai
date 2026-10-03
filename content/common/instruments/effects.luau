--!strict
-- The instruments' effects (`off_80D4298`), every third tick of playing:
-- Discord's, Timpani's and Silence's, each a one-tick hit of the sixth
-- element's damage 0 that gives a status. A game gives its status and
-- collision types (BN6's rules/status and rules/collision; BN5's own).

local regions = require("@common/regions")

local effects = {}

-- The sixth element: the hit's damage is 0.
local ELEMENT_NONE = 5

local function hit(me: Object, region: Region, is: CollisionType, hits: CollisionType, status: Status)
    battle.hitbox(me, {
        panel_x = me.panel_x,
        panel_y = me.panel_y,
        element = ELEMENT_NONE,
        z = 0,
        region = region,
        -- None.
        hit_effect = nil,
        target = hits,
        self_type = is,
        damage = 0,
        stamina = 0,
        hit_mod = 0,
        status = status,
    })
end

-- `sub_80D4392` (Discord's): a hit on the other navi's panel that gives
-- `status` (confusion), and learns nothing (`hits`: nothing).
function effects.confuse(status: Status, is: CollisionType, hits: CollisionType): (me: Object) -> ()
    return function(me: Object)
        hit(me, regions.other_navi[me.alliance], is, hits, status)
    end
end

-- `sub_80D43C0` (Timpani's): a hit over the solid panels that gives
-- `status` (immobilized) to the other side, and a camera shake.
function effects.drum(status: Status, is: CollisionType, hits: CollisionType): (me: Object) -> ()
    return function(me: Object)
        hit(me, regions.solid, is, hits, status)
        battle.shake_camera(2, 4)
    end
end

-- `sub_80D43E8` (Silence's): a hit over the whole field that gives
-- `status` (blindness) to the other side.
function effects.hush(status: Status, is: CollisionType, hits: CollisionType): (me: Object) -> ()
    return function(me: Object)
        hit(me, regions.field, is, hits, status)
    end
end

return effects
