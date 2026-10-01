--!strict
-- A falling meteor's panel marker (the original's effect object #0x35,
-- `sub_80E4344`; docs/engine/dimming-chip-effects.md §6), which the meteor
-- (./falling_meteor) spawns, whoever drops it: it highlights a region
-- around its panel in blinks that come faster (by its look's shrink), for
-- its look's lifetime, or until the battle is over. Drawn only.

local marker = {}

-- How a marker blinks (`byte_80E4334`, by the original's first
-- parameter): its lifetime, its blink period, the ticks lit each period,
-- and how much each period shrinks the next (doubling).
export type MarkerLook = {
    lifetime: number,
    period: number,
    lit: number,
    shrink: number,
}

export type State = {
    look: MarkerLook,
    -- The region it highlights (the original's second parameter).
    region: Region?,
    -- The ticks to the next period, lit ticks left, the next shrink
    -- (its second to fourth parameters, once it runs).
    period_left: number,
    lit_left: number,
    shrink: number,
}

-- A meteor's: 45 ticks, lit 5 of every 10.
marker.meteor = define.record("marker-look", { lifetime = 45, period = 10, lit = 5, shrink = 0 } :: MarkerLook)

-- `object_highlightPanelRegion`: the region's panels from (x, y), turned
-- toward `side`'s front.
local function highlight(x: number, y: number, r: Region?, side: number)
    if not r then
        return
    end
    local dir = 1 - 2 * side
    for _, p in r.panels or {} do
        local px, py = x + p[1] * dir, y + p[2]
        if field.valid(px, py) then
            field.highlight(px, py)
        end
    end
end

-- `sub_80E43A8`: gone at its lifetime's end, or when the battle is over
-- (unless dimmed), after this tick's blink.
local function tick(me: Object)
    local s = me.state :: State
    if not battle.dimmed() and battle.over() then
        me:set_lifecycle("destroy")
    else
        local t = me.timer - 1
        me.timer = t
        if t == 0 then
            me:set_lifecycle("destroy")
        end
    end
    if s.lit_left > 0 then
        s.lit_left -= 1
        highlight(me.panel_x, me.panel_y, s.region, me.alliance)
    end
    s.period_left = int.u8(s.period_left - 1)
    if s.period_left == 0 then
        s.lit_left = s.look.lit
        s.period_left = int.u8(s.look.period - s.shrink)
        s.shrink = int.u8(s.shrink * 2)
    end
end

-- `sub_80E4364`: on the panel its spawner gave as its position; off the
-- field, freed.
local function init(me: Object)
    me.panel_x, me.panel_y = int.u8(me.pos.x), int.u8(me.pos.y)
    if not field.valid(me.panel_x, me.panel_y) then
        me:free()
        return
    end
    local s = me.state :: State
    me.timer = s.look.lifetime
    s.period_left, s.lit_left, s.shrink = s.look.period, s.look.lit, s.look.shrink
    me:set_lifecycle("update")
    tick(me)
end

marker.kind = define.kind {
    id = "meteors/marker",
    pool = "effect",
    state = {
        look = "record:marker-look",
        region = "region",
        period_left = "u8",
        lit_left = "u8",
        shrink = "u8",
    },
    -- `sub_80E4344`.
    update = function(me: Object)
        local l = me.lifecycle
        if l == "init" then
            init(me)
        elseif l == "update" then
            tick(me)
        else
            me:free()
        end
    end,
}

-- `sub_80E43F6`: a marker of `look` highlighting `region` on (x, y), of
-- `owner`'s side. (Its Z is the spawner's register, which nothing reads.)
function marker.spawn(owner: Object, x: number, y: number, look: MarkerLook, region: Region?): Object?
    local o = battle.spawn(marker.kind, Vec3.new(x, y, 0))
    if not o then
        return nil
    end
    local s = o.state :: State
    s.look, s.region = look, region
    o.alliance = owner.alliance
    return o
end


return marker
