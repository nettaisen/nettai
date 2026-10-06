--!strict
-- A shower of Fire meteors (the original's effect object #0x46,
-- `sub_80E59E8`) and instant chip effect 16 (`sub_80E5A64`): every 20
-- ticks, a number of times (the game's Param2), a meteor (Meteors'
-- falling meteor, chips/meteors/falling_meteor) falls on one of the other
-- side's navis still standing, picked at random, or on a panel next to it
-- (6 draws in 16).
-- (It drops the meteor's row 2 for three drops, else 0: EXE6's rows are
-- alike.)
--
-- No chip names effect 16. The patch cards' meteors (lib/patch_cards/
-- meteors, MettFire's B button and Metrid's B+Back) spawn the shower from
-- an action of their own; link navis' actions would too.

local falling_meteor = require("../../chips/meteors/falling_meteor")
local shower = require("@exelib/meteors/shower")

local meteor_shower = {}

export type State = {
    -- The navi it last picked (ExtraVars).
    target: Object?,
    -- How many meteors it drops (the game's Param2).
    drops: number,
}

local FIRE = 1
local DROP_TICKS = 0x14
-- `sub_80E5A8A`: one of the other side's navis still standing, at random;
-- `sub_80E5AD6`: the target's panel, or a neighbor (@exelib/meteors/shower).
meteor_shower.pick = shower.pick
meteor_shower.aim = shower.aim

-- `sub_80CF594`: a meteor on (x, y). (Its X and Y are the registers'
-- values, which its init replaces.)
local function meteor(me: Object, x: number, y: number)
    local damage = bit32.bor(me.damage, bit32.lshift(me.stamina, 16))
    local row = if (me.state :: State).drops == 3 then 2 else 0
    falling_meteor.spawn(row, me, x, y, me.element, damage, false, Vec3.new(y, me.element, 0))
end

-- `sub_80E5A22`.
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
    me.timer = DROP_TICKS
    local s = me.state :: State
    s.target = meteor_shower.pick(me)
    local x, y = meteor_shower.aim(s.target)
    meteor(me, x, y)
    local drops = me.timer2 - 1
    me.timer2 = drops
    if drops <= 0 then
        me:set_lifecycle("destroy")
    end
end

meteor_shower.kind = new.kind {
    id = "meteor-shower",
    pool = "effect",
    state = { target = "object", drops = "u8" },
    -- `sub_80E59E8`.
    update = function(me: Object)
        local l = me.lifecycle
        if l == "init" then
            -- sub_80E5A08
            local s = me.state :: State
            s.target = meteor_shower.pick(me)
            me.timer2 = s.drops
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

-- `sub_80E5A64`: a shower of `user`'s of `drops` meteors, Fire, with the
-- damage word `damage`, at `pos` (the registers' values: it has no
-- position of its own). It doesn't run while dimmed.
function meteor_shower.spawn(user: Object, pos: Vec3, drops: number, damage: number): Object?
    local o = battle.spawn(meteor_shower.kind, pos)
    if o then
        o.alliance, o.flip = user.alliance, user.flip
        o.related1 = user
        o:set_damage_word(damage)
        o.element = FIRE
        o.run_while_dimmed = false;
        (o.state :: State).drops = drops
    end
    return o
end

-- Instant chip effect 16: an `instant` hook that starts a shower of the
-- user's of `drops` meteors (at the instant chips' action's registers).
function meteor_shower.instant(drops: number): (user: Object, spec: InstantChipSpec) -> ()
    return function(user: Object, spec: InstantChipSpec)
        meteor_shower.spawn(user, Vec3.new(spec.panel_y, spec.element, spec.z), drops, spec.damage)
    end
end

return meteor_shower
