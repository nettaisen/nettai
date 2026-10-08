--!strict
-- EXE5's falling meteor (attack object #0x56) and meteor shower (effect object #0x46): the shared code
-- (@exelib/meteors: EXE6's) but for
--
-- - the meteor's look and rows (0x080D0B24: rows 0 and 1 EXE6's, row 1 the Meteors chip's; row 3, the shower's,
--   cracks its panel; row 4, DarkMetr's, hits as EXE5's row 0x2B with the bug 0x19 (argument 1), shakes the cameras
--   at 2 and breaks its panel; row 2, which hits with modifier 0, nothing drops);
-- - the shower (0x080E58A8, 0x080E58EC), its chip's variant (`meteors.variant`; its first parameter set gives
--   DarkMetr's): DarkMetr's drops 10 more meteors and 2 for each of the first three reactions its side's navi took
--   (side stat 3), one every 10 ticks (Astroid's 20), on the target's own panel (no draw for a neighbor), row 4
--   (Astroid's row 3). (Its init also gives the shower, which has no collision, the bug 0x19: EXE5 writes it through
--   a null pointer, into the BIOS, to no effect.)
--
-- Not ported: in the operation battle (never in a netbattle), a meteor also spawns effect object 0x83 on its panel
-- for 20 ticks (0x080E9FA4, as FireHit's warning and the swords' swing).

local collision = require("../../rules/collision")
local effects = require("../../lib/effects")
local falling_meteor = require("@exelib/meteors/falling_meteor")
local marker = require("./marker")
local meteor_shower = require("@exelib/meteors/shower")
local sparks = require("../../lib/sparks")

local meteors = {}

-- The Meteors chip's row (row 1). (The showers' are their chips': chips/astroid, chips/darkmetr.)
meteors.row = falling_meteor.row { is = collision.thrown, hits = collision.hits_navis, hit_mod = 3, shake = 1, panel = "none" }

meteors.kind = falling_meteor.make_kind("meteor", {
    sprite = asset.sprite("meteor"),
    sounds = { fall = asset.sound("justice-one"), land = asset.sound("hit-bomb-1") },
    spark = sparks.of_element,
    -- (Its landing's burst EXE6's.)
    burst = effects.burst,
    marker = marker,
    hit_on = "burst",
    marker_takes_side = true,
})

-- A chip's shower (its first parameter's pick): the ticks between its meteors, their row, where each falls for the
-- navi picked (`meteors.aim`: on or beside it), and the meteors it drops more than its chip says (none: none).
export type ShowerVariant = {
    period: number,
    row: MeteorRow,
    aim: (target: Object?) -> (number, number),
    more: ((me: Object) -> number)?,
}

function meteors.variant(v: ShowerVariant): ShowerVariant
    return new.record("exe5-meteor-shower", v)
end

-- The target's panel, or on some draws a neighbor of it (@exelib/meteors/shower).
meteors.aim = meteor_shower.aim

export type ShowerState = {
    -- The navi it last picked (ExtraVars).
    target: Object?,
    -- Its chip's variant (its first parameter), and the meteors it drops (its second).
    variant: ShowerVariant,
    drops: number,
}

local FIRE = 1

-- 0x080E58EC
local function tick(me: Object)
    if battle.over() then
        me:set_lifecycle("destroy")
        return
    end
    local left = me.timer - 1
    me.timer = left
    if left > 0 then
        return
    end
    local s = me.state :: ShowerState
    local v = s.variant
    me.timer = v.period
    s.target = meteor_shower.pick(me)
    local x, y = v.aim(s.target)
    local damage = bit32.bor(me.damage, bit32.lshift(me.stamina, 16))
    falling_meteor.spawn_kind(meteors.kind, v.row, me, x, y, me.element, damage, false, Vec3.new(y, me.element, 0))
    local drops = me.timer2 - 1
    me.timer2 = drops
    if drops <= 0 then
        me:set_lifecycle("destroy")
    end
end

meteors.shower = new.kind {
    id = "meteors/shower",
    pool = "effect",
    state = { target = "object", variant = "record:exe5-meteor-shower", drops = "u8" },
    -- 0x080E5888
    update = function(me: Object)
        local l = me.lifecycle
        if l == "init" then
            -- 0x080E58A8
            local s = me.state :: ShowerState
            s.target = meteor_shower.pick(me)
            local more = s.variant.more
            me.timer2 = s.drops + (if more then more(me) else 0)
            me.timer = 0
            me:set_lifecycle("update")
            tick(me)
        elseif l == "update" then
            tick(me)
        else
            me:free()
        end
    end,
}

-- 0x080E5944, instant chip effect 17: a shower of the user's, of `variant`, of `drops` meteors (and its variant's
-- more), Fire, not running while dimmed.
function meteors.instant(variant: ShowerVariant, drops: number): (user: Object, spec: InstantChipSpec) -> ()
    return function(user: Object, spec: InstantChipSpec)
        local o = battle.spawn(meteors.shower, Vec3.new(spec.panel_y, spec.element, spec.z))
        if o then
            o.alliance, o.flip = user.alliance, user.flip
            o.related1 = user
            o:set_damage_word(spec.damage)
            o.element = FIRE
            o.run_while_dimmed = false
            local s = o.state :: ShowerState
            s.variant, s.drops = variant, drops
        end
    end
end

return meteors
