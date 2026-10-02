--!strict
-- A shower of Fire meteors (the original's effect object #0x46,
-- `sub_80E59E8`) and instant chip effect 16 (`sub_80E5A64`): every 20
-- ticks, a number of times (the game's Param2), a meteor (Meteors'
-- falling meteor, chips/meteors/falling_meteor) falls on one of the other
-- side's navis still standing, picked at random, or on a panel next to it
-- (6 draws in 16).
-- (The game drops the meteor's look 2 for three drops, else 0: the rows
-- are the same.)
--
-- No chip names effect 16. The patch cards' meteors (lib/patch-cards/
-- meteors, MettFire's B button and Metrid's B+Back) spawn the shower from
-- an action of their own; link navis' actions would too.

local falling_meteor = require("../../chips/meteors/falling_meteor")

local meteor_shower = {}

export type State = {
    -- The navi it last picked (ExtraVars).
    target: Object?,
    -- How many meteors it drops (the game's Param2).
    drops: number,
}

local FIRE = 1
local DROP_TICKS = 0x14
-- Whether to pick a panel next to the target, by a draw's low four bits
-- (`byte_80E5B50`, running into `byte_80E5B58`), and the neighbours
-- (`byte_80E5B58`, its last pair the next routine's code).
local NEAR = { [0] = false, true, true, false, false, false, false, false, false, true, false, true, true, false, true, false }
local NEIGHBOURS = { { 0, -1 }, { 0, 1 }, { -1, 0 }, { 1, 0 } }

-- `sub_80E5A8A`: one of the other side's navis still standing, at random
-- (a draw only if there is one).
local function pick(me: Object): Object?
    local standing = {}
    for _, o in battle.alive_actors(bit32.bxor(me.alliance, 1)) do
        if o.hp ~= 0 then
            table.insert(standing, o)
        end
    end
    if #standing == 0 then
        return nil
    end
    return standing[battle.rng_positive() % #standing + 1]
end

-- `sub_80E5AD6`: the target's panel, or on some draws a neighbour of it on
-- the field (another draw picks which).
local function aim(target: Object?): (number, number)
    if target == nil then
        error("meteor shower without a target (sub_80E5AD6 reads a panel through a null pointer)")
    end
    local x, y = target.panel_x, target.panel_y
    if not NEAR[bit32.band(battle.rng_positive(), 0xF)] then
        return x, y
    end
    local near = {}
    for _, d in NEIGHBOURS do
        local nx, ny = x + d[1], y + d[2]
        if field.valid(nx, ny) then
            table.insert(near, bit32.bor(bit32.band(nx, 0xFF), bit32.lshift(ny, 4)))
        end
    end
    if #near == 0 then
        return x, y
    end
    local b = bit32.band(near[battle.rng_positive() % #near + 1], 0xFF)
    return bit32.band(b, 7), bit32.rshift(b, 4)
end

-- `sub_80CF594`: a meteor on (x, y). (Its X and Y are the registers'
-- values, which its init replaces.)
local function meteor(me: Object, x: number, y: number)
    local damage = bit32.bor(me.damage, bit32.lshift(me.stamina, 16))
    falling_meteor.spawn(me, x, y, me.element, damage, false, Vec3.new(y, me.element, 0))
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
    s.target = pick(me)
    local x, y = aim(s.target)
    meteor(me, x, y)
    local drops = me.timer2 - 1
    me.timer2 = drops
    if drops <= 0 then
        me:set_lifecycle("destroy")
    end
end

meteor_shower.kind = define.kind {
    id = "meteor-shower",
    pool = "effect",
    state = { target = "object", drops = "u8" },
    -- `sub_80E59E8`.
    update = function(me: Object)
        local l = me.lifecycle
        if l == "init" then
            -- sub_80E5A08
            local s = me.state :: State
            s.target = pick(me)
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
