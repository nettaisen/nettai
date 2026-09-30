--!strict
-- A dust storm (attack object #0x0E, `sub_80C6280`) and instant chip
-- effect 17 (`sub_80C6330`): on the user's panel it raises dust motes
-- (objects/dust-storm-mote) on the nine panels around, and hits all of
-- them (region 0xF) every 12 ticks, for Param2 ticks, or until the user
-- leaves the action Param3 ties it to (Param3 2: none; its hits then land
-- while dimmed). No chip or weapon of MegaMan's names effect 17; link
-- navis' actions spawn the storm themselves.

local dust_storm_mote = require("../dust-storm-mote/dust_storm_mote")

local dust_storm = {}

-- The actions the user must stay in, by Param3 (`dword_80C62F4`).
local TIED = { [0] = 0x0D, 0x0E, 0x00, 0x5E }
local UNTIED = 2
-- The motes' panels (`byte_80C639C`).
local AROUND = { { 0, 0 }, { -1, -1 }, { 0, -1 }, { 1, -1 }, { -1, 0 }, { 1, 0 }, { -1, 1 }, { 0, 1 }, { 1, 1 } }
-- The hit (`dword_80C6410`): region, hit spark, collision types, hit
-- modifier; every 12 ticks.
local REGION, TARGET, SELF_TYPE, HIT_MOD = 0xF, 0, 4, 1
local HIT_TICKS = 0xC

-- `sub_80C6330`, instant chip effect 17: a storm on the user's panel; it
-- runs while dimmed. (It spawns at the registers' values, which its init
-- replaces but for Z's fraction.)
function dust_storm.instant_chip(user: Object, spec: InstantChipSpec)
    local o = battle.spawn_kind("dust-storm", Vec3.new(spec.panel_y, spec.element, spec.z), spec.params)
    if o then
        o.panel_x, o.panel_y = spec.panel_x, spec.panel_y
        o.element = spec.element
        o.related1 = user
        o.damage = spec.damage
        o.stamina = bit32.rshift(spec.damage, 16)
        o.alliance, o.flip = user.alliance, user.flip
        o.run_while_dimmed = true
    end
end

-- `sub_80C6358`: motes on the panels around, on the field.
local function raise(me: Object)
    for _, d in AROUND do
        local x, y = me.panel_x + d[1], me.panel_y + d[2]
        if field.valid(x, y) then
            dust_storm_mote.spawn(me, me.related1, x, y, me:param(2), me:param(3))
        end
    end
end

-- `sub_80C63AE`: the hit.
local function hit(me: Object)
    local box = battle.hitbox(me, {
        panel_x = me.panel_x,
        panel_y = me.panel_y,
        element = me.element,
        z = me.pos.z,
        region = REGION,
        hit_effect = 0xFF,
        target = TARGET,
        self_type = SELF_TYPE,
        damage = me.damage,
        stamina = me.stamina,
        hit_mod = HIT_MOD,
    })
    -- sub_80C53A6
    if box and me:param(3) == UNTIED then
        box.run_while_dimmed = true
    end
end

-- `sub_80C62F8`.
local function storm(me: Object)
    if me.phase == 0 then
        me.phase = 4
        me.timer = me:param(2)
        raise(me)
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
    hit(me)
end

-- `sub_80C62B0`.
local function tick(me: Object)
    if battle.over() then
        me:set_lifecycle("destroy")
        return
    end
    local mode = me:param(3)
    if mode ~= UNTIED then
        local action = TIED[mode]
        if action == nil then
            error(string.format("dust storm mode %d reads past dword_80C62F4", mode))
        end
        if (me.related1 :: Object).action ~= action then
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
    storm(me)
end

-- `sub_80C6280`.
function dust_storm.update(me: Object)
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
end

return dust_storm
