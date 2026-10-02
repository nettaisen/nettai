--!strict
-- A dust storm (the original's attack object #0x0E, `sub_80C6280`) and
-- instant chip effect 17 (`sub_80C6330`): on the user's panel it raises
-- dust motes (dust_storm_mote.luau) on the nine panels around, and hits
-- all of them every 12 ticks, for a number of ticks (the game's Param2),
-- or until the user leaves the action it is tied to. An untied storm (the
-- game's Param3 2) lands its hits while dimmed; a tied one stands still
-- while dimmed. (The game names the action by Param3 from
-- `dword_80C62F4`: 0x0D, 0x0E and 0x5E for modes 0, 1 and 3, link navis'
-- actions; here the spawner passes the action.)
--
-- No chip or weapon of MegaMan's names effect 17; link navis' actions
-- spawn the storm themselves, with `dust_storm.instant { ... }` when they
-- are ported (unverified: no scenario reaches it). Count's rain
-- (chips/count/navi, the Japanese games') is an untied storm over the
-- middle of the other side's area, `dust_storm.spawn`.

local collision = require("../../rules/collision")
local regions = require("../regions")
local mote = require("./dust_storm_mote")

local dust_storm = {}

-- How a storm blows: for `ticks` ticks, while the user runs `tied` (nil:
-- untied).
export type Storm = DustStorm

export type State = {
    ticks: number,
    tied: Action?,
}

-- The motes' panels (`byte_80C639C`).
local AROUND = { { 0, 0 }, { -1, -1 }, { 0, -1 }, { 1, -1 }, { -1, 0 }, { 1, 0 }, { -1, 1 }, { 0, 1 }, { 1, 1 } }
-- The hit (`dword_80C6410`): the 3x3 around it, no spark, an attack whose
-- target reaches nothing (it learns nothing of its hits; the navis there
-- react to it and take them), hit modifier 1; every 12 ticks.
local NO_SPARK: Spark?, HIT_MOD = nil, 1
local HIT_TICKS = 0xC

-- `sub_80C6358`: motes on the panels around, on the field.
local function raise(me: Object, s: State)
    for _, d in AROUND do
        local x, y = me.panel_x + d[1], me.panel_y + d[2]
        if field.valid(x, y) then
            mote.spawn(me, me.related1, x, y, s.ticks, s.tied)
        end
    end
end

-- `sub_80C63AE`: the hit.
local function hit(me: Object, s: State)
    local box = battle.hitbox(me, {
        panel_x = me.panel_x,
        panel_y = me.panel_y,
        element = me.element,
        z = me.pos.z,
        region = regions.square,
        hit_effect = NO_SPARK,
        target = collision.nothing,
        self_type = collision.attack,
        damage = me.damage,
        stamina = me.stamina,
        hit_mod = HIT_MOD,
    })
    -- sub_80C53A6
    if box and s.tied == nil then
        box.run_while_dimmed = true
    end
end

-- `sub_80C62F8`.
local function storm(me: Object, s: State)
    if me.phase == 0 then
        me.phase = 4
        me.timer = s.ticks
        raise(me, s)
        me.timer2 = 0
    end
    local left = me.timer - 1
    me.timer = left
    if left <= 0 then
        me:set_lifecycle("destroy")
        return
    end
    local next = me.timer2 - 1
    me.timer2 = next
    if next > 0 then
        return
    end
    me.timer2 = HIT_TICKS
    hit(me, s)
end

-- `sub_80C62B0`.
local function tick(me: Object)
    if battle.over() then
        me:set_lifecycle("destroy")
        return
    end
    local s = me.state :: State
    if s.tied ~= nil then
        if (me.related1 :: Object):navi_action() ~= s.tied then
            me:set_lifecycle("destroy")
            return
        end
        if battle.dimmed() then
            return
        end
    end
    local a = me.action
    if a ~= 0 then
        error(string.format("dust storm action %#x reads past its table", a))
    end
    storm(me, s)
end

dust_storm.kind = define.kind {
    id = "dust-storm",
    pool = "attack",
    state = { ticks = "u16", tied = "action" },
    -- `sub_80C6280`.
    update = function(me: Object)
        local l = me.lifecycle
        if l == "init" then
            -- sub_80C62A0: on its panel, Z's whole pixels cleared.
            me:set_coordinates_from_panel()
            local p = me.pos
            me.pos = Vec3.new(p.x, p.y, bit32.band(p.z, 0xFFFF))
            me:set_lifecycle("update")
        elseif l == "update" then
            tick(me)
        else
            me:destroy()
        end
    end,
}

-- `sub_80C6330`: a storm of `user`'s on (x, y) with the damage word
-- `damage`, blowing as `storm` says; it runs while dimmed. (It spawns at
-- the registers' values, the row, the element and `z`, which its init
-- replaces but for Z's fraction.)
function dust_storm.spawn(user: Object, x: number, y: number, z: number, element: number, damage: number, storm_: Storm): Object?
    local o = battle.spawn(dust_storm.kind, Vec3.new(y, element, z))
    if o then
        o.panel_x, o.panel_y = x, y
        o.element = element
        o.related1 = user
        o:set_damage_word(damage)
        o.alliance, o.flip = user.alliance, user.flip
        o.run_while_dimmed = true
        local s = o.state :: State
        s.ticks, s.tied = storm_.ticks, storm_.tied
    end
    return o
end

-- Instant chip effect 17: an `instant` hook that raises a storm of the
-- user's on its panel (`sub_80C6330`).
function dust_storm.instant(storm_: Storm): (user: Object, spec: InstantChipSpec) -> ()
    return function(user: Object, spec: InstantChipSpec)
        dust_storm.spawn(user, spec.panel_x, spec.panel_y, spec.z, spec.element, spec.damage, storm_)
    end
end

return dust_storm
