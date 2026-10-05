--!strict
-- EXE5's falling meteor (attack object #0x56) and meteor shower (effect object #0x46): the shared code
-- (@exelib/meteors: EXE6's) but for
--
-- - the meteor's look and rows (0x080D0B24: rows 0 to 2 EXE6's, but row 2 hits with modifier 0, 0x080D0BE4; row 3
--   cracks its panel; row 4, DarkMetr's, hits as EXE5's row 0x2B with the bug 0x19 (argument 1), shakes the cameras
--   at 2 and breaks its panel);
-- - the shower (0x080E58A8, 0x080E58EC) with its first parameter set (DarkMetr's): 10 more meteors and 2 for each of
--   the first three reactions its side's navi took (side stat 3), one every 10 ticks (20 without), on the target's
--   own panel (no draw for a neighbor), row 4 (row 3 without). (Its init also gives the shower, which has no
--   collision, the bug 0x19: EXE5 writes it through a null pointer, into the BIOS, to no effect.)
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

local ROW: MeteorRow = { is = collision.thrown, hits = collision.hits_navis, hit_mod = 3, shake = 1, panel = "none" }

meteors.kind = falling_meteor.make_kind("meteor", {
    sprite = asset.sprite("meteor"),
    sounds = { fall = asset.sound("justice-one"), land = asset.sound("hit-bomb-1") },
    rows = {
        [0] = ROW,
        [1] = ROW,
        [2] = { is = collision.thrown, hits = collision.hits_navis, hit_mod = 0, shake = 1, panel = "none" },
        [3] = { is = collision.thrown, hits = collision.hits_navis, hit_mod = 3, shake = 1, panel = "crack" },
        [4] = {
            is = collision.collision_2b,
            hits = collision.hits_navis,
            hit_mod = 3,
            shake = 2,
            panel = "break",
            bug = 0x19,
            bug_arg = 1,
        },
    },
    spark = sparks.of_element,
    -- (Its landing's burst EXE6's.)
    burst = effects.burst,
    marker = marker,
})

export type ShowerState = {
    -- The navi it last picked (ExtraVars).
    target: Object?,
    -- Its first and second parameters: DarkMetr's (1), and the meteors it drops on top of the dark's.
    dark: boolean,
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
    me.timer = if s.dark then 10 else 20
    s.target = meteor_shower.pick(me)
    local x, y
    if s.dark then
        local t = s.target
        if not t then
            error("DarkMetr's meteor shower without a target (0x080E591E reads a panel through a null pointer)")
        end
        x, y = t.panel_x, t.panel_y
    else
        x, y = meteor_shower.aim(s.target)
    end
    local damage = bit32.bor(me.damage, bit32.lshift(me.stamina, 16))
    falling_meteor.spawn_kind(meteors.kind, if s.dark then 4 else 3, me, x, y, me.element, damage, false, Vec3.new(y, me.element, 0))
    local drops = me.timer2 - 1
    me.timer2 = drops
    if drops <= 0 then
        me:set_lifecycle("destroy")
    end
end

meteors.shower = define.kind {
    id = "meteors/shower",
    pool = "effect",
    state = { target = "object", dark = "bool", drops = "u8" },
    -- 0x080E5888
    update = function(me: Object)
        local l = me.lifecycle
        if l == "init" then
            -- 0x080E58A8
            local s = me.state :: ShowerState
            s.target = meteor_shower.pick(me)
            local more = 0
            if s.dark then
                more = math.min(battle.side_stat(me.alliance, 3), 3) * 2 + 10
            end
            me.timer2 = s.drops + more
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

-- 0x080E5944, instant chip effect 17: a shower of the user's (DarkMetr's with `dark`) of `drops` meteors on top of
-- the dark's, Fire, not running while dimmed.
function meteors.instant(dark: boolean, drops: number): (user: Object, spec: InstantChipSpec) -> ()
    return function(user: Object, spec: InstantChipSpec)
        local o = battle.spawn(meteors.shower, Vec3.new(spec.panel_y, spec.element, spec.z))
        if o then
            o.alliance, o.flip = user.alliance, user.flip
            o.related1 = user
            o:set_damage_word(spec.damage)
            o.element = FIRE
            o.run_while_dimmed = false
            local s = o.state :: ShowerState
            s.dark, s.drops = dark, drops
        end
    end
end

return meteors
