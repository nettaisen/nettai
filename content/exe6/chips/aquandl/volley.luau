--!strict
-- AquaNdl's volley (effect object #0x40, `sub_80E50C4`): an unseen
-- launcher that picks three of the enemy's panels and drops an aqua needle
-- (attack object #0x50, chips/aquandl/needle.luau) on each, 8 ticks apart.
-- It aims at the enemies standing when it starts: the first two it finds
-- (viruses, NameIDs 0 to 0xAE, before navis, 0x100 to 0x1C3), and the
-- third at the healthier of two (at random if even); a lone enemy gets
-- all three. The panels go farthest first, then top to bottom. The
-- needles wear the chip's palette. See docs/engine.
--
-- Its position is the registers its spawner leaves (a code address among
-- them): it never draws, and nothing reads it (compat's
-- `scratch_position`).

local needle = require("./needle")

local volley = {}

export type State = {
    -- The panels it aims at, x | y << 4 each, in the order they get
    -- needles, and how many it has dropped.
    targets: { number },
    next: number,
    -- Needles left to drop (the original keeps the count in its second
    -- parameter), and their palette (its first).
    left: number,
    palette: number,
}

-- Needles to drop, and the ticks between them.
local NEEDLES, INTERVAL = 3, 8
-- The enemies it aims at, by what they are taken for: viruses (the
-- original's NameIDs up to 0xAE, short of the last virus, 0xBA: an object
-- with no identity is its NameID 0), then navis (0x100 to 0x1C3).
local NAVIS: { [string]: boolean } = {
    navi = true,
    player = true,
}

local function is_virus(o: Object): boolean
    local identity = o.identity
    return identity == nil or identity.class == "virus"
end

local function is_navi(o: Object): boolean
    local identity = o.identity
    return identity ~= nil and NAVIS[identity.class] == true
end

-- `object_getEnemyByNameRange`: `side`'s combatants that `wanted` takes,
-- added to `into` in slot order.
local function enemies_named(side: number, wanted: (o: Object) -> boolean, into: { Object })
    for _, o in battle.alive_actors(side) do
        if wanted(o) then
            table.insert(into, o)
        end
    end
end

-- `sub_80E5208`: two target panels in firing order, farthest (for its
-- facing) first, then the lower byte (the top row) first.
local function ordered(a: number, b: number, right: boolean): (number, number)
    local ax, bx = bit32.band(a, 0xF), bit32.band(b, 0xF)
    if right then
        if ax > bx then
            return a, b
        elseif ax < bx then
            return b, a
        end
    else
        if ax < bx then
            return a, b
        elseif ax > bx then
            return b, a
        end
    end
    if a <= b then
        return a, b
    end
    return b, a
end

-- `sub_80E5174`: pick the three targets and sort them.
local function aim(me: Object, s: State)
    local side = bit32.bxor(me.alliance, 1)
    local found: { Object } = {}
    enemies_named(side, is_virus, found)
    enemies_named(side, is_navi, found)
    local picks: { Object }
    if #found == 0 then
        error("AquaNdl's volley (sub_80E5174) found no enemy and reads its stack")
    elseif #found == 1 then
        picks = { found[1], found[1], found[1] }
    elseif #found == 2 then
        local a, b = found[1], found[2]
        local third
        if a.hp < b.hp then
            third = b
        elseif a.hp > b.hp then
            third = a
        else
            third = if bit32.btest(battle.rng_positive(), 4) then b else a
        end
        picks = { a, b, third }
    else
        picks = { found[1], found[2], found[3] }
    end
    local t: { number } = {}
    for i = 1, NEEDLES do
        local o = picks[i]
        t[i] = bit32.band(bit32.bor(o.panel_x, bit32.lshift(o.panel_y, 4)), 0xFF)
    end
    local right = bit32.bxor(me.alliance, me.flip) == 0
    for i = 1, NEEDLES - 1 do
        for j = i + 1, NEEDLES do
            t[i], t[j] = ordered(t[i], t[j], right)
        end
    end
    for i = 1, NEEDLES do
        s.targets[i] = t[i]
    end
    s.next = 0
end

-- `sub_80E50FE`: drop the next needle, then wait 8 ticks; gone after the
-- third, or when the battle is over.
local function tick(me: Object)
    if battle.over() then
        me:set_lifecycle("destroy")
        return
    end
    local s = me.state :: State
    if me.action == 0 then
        me.action = 1
        local i = s.next
        if i >= NEEDLES then
            error("AquaNdl's volley (sub_80E50FE) reads past its targets")
        end
        s.next = i + 1
        local p = s.targets[i + 1]
        -- (The needles never crack their panels: the volley passes its
        -- first parameter alone.)
        needle.spawn(me, bit32.band(p, 7), bit32.rshift(p, 4), me.element, s.palette, false, me.damage + me.stamina * 0x10000)
        me.timer = INTERVAL
    end
    local t = me.timer - 1
    me.timer = t
    if t > 0 then
        return
    end
    local left = s.left - 1
    s.left = left
    if left == 0 then
        me:set_lifecycle("destroy")
    else
        me.action = 0
    end
end

-- `sub_80E50E4`.
local function init(me: Object)
    local s = me.state :: State
    aim(me, s)
    s.left = NEEDLES
    me:set_lifecycle("update")
    tick(me)
end

volley.kind = new.kind {
    id = "aquandl/volley",
    pool = "effect",
    state = { targets = "u8[3]", next = "u8", left = "u8", palette = "u8" },
    -- `sub_80E50C4`.
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

-- `sub_80E5150`: a volley for `user`'s side, with the damage word `damage`
-- and its needles' `palette`. It doesn't run while dimmed.
function volley.spawn(user: Object, element: number, palette: number, damage: number): Object?
    local o = battle.spawn(volley.kind, Vec3.zero)
    if o then
        o.related1 = user
        o.element = element
        o.alliance, o.flip = user.alliance, user.flip
        o:set_damage_word(damage)
        o.run_while_dimmed = false
        local s = o.state :: State
        s.palette = palette
    end
    return o
end

return volley
