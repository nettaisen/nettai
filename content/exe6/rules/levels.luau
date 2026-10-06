--!strict
-- What a navi code's level gives a navi's stats (docs/engine/link-navis.md): the navi's `levels` (its
-- definition's), which the save's reload reads (`reloadCurNaviBaseStats_8120df0`) for a link navi and the NaviCust's
-- reload (`reloadCurNaviStatBoosts`: `sub_8121154` after `sub_813C458`) for MegaMan from a navi code.

local levels = {}

-- The story's progress a match assumes (`sub_8121108`'s index, 0 to 6): the game cleared, every flag set, as the
-- chip lab's saves and Tango's have it. A link navi's base HP is its `levels.base_hp` at it.
levels.CLEARED = 6

-- `sub_8121154` with `sub_8123208`: what level `level` adds to `stats` (the navi's `levels.by_level`): its HP to the
-- maximum (not the base HP); the buster's Attack, Rapid and Charge, to 4 at most, the Mega level, to 10, and the
-- custom level, to 8 (each clamped even when the level adds nothing); SuperArmor, FloatShoes and AirShoes; the
-- B+Back special it names. Nothing for a navi without levels or a level past them.
function levels.add(stats: Navi, level: number)
    local own = stats.navi.levels
    if own == nil then
        return
    end
    local g = own.by_level[level + 1]
    if g == nil then
        return
    end
    stats.max_hp = (stats.max_hp + g.hp) % 0x10000
    stats.attack = math.min(stats.attack + (g.attack or 0), 4)
    stats.rapid = math.min(stats.rapid + (g.rapid or 0), 4)
    stats.charge = math.min(stats.charge + (g.charge or 0), 4)
    stats.mega_level = math.min(stats.mega_level + (g.mega_level or 0), 10)
    stats.custom_level = math.min(stats.custom_level + (g.custom_level or 0), 8)
    if g.air_shoes then
        stats.air_shoes = true
    end
    if g.float_shoes then
        stats.float_shoes = true
    end
    if g.super_armor then
        stats.super_armor = true
    end
    local back_special: Weapon? = g.back_special
    if back_special ~= nil then
        stats.back_special_weapon = back_special
    end
end

-- `reloadCurNaviBaseStats_8120df0` for a link navi (one with `levels` that doesn't change form) at `level`, over
-- its fresh stats with what the save keeps (the setup's: the Regular memory, which the save system writes, and the
-- folder's bytes, which no battle reads), the game cleared and in the real world: the base and maximum HP by the
-- story's progress (`off_8120F44`), the level's gains (`levels.add`), and the HP its maximum (as
-- `reloadCurNaviStatBoosts_813c3ac` leaves it). The original runs it when a navi code is received or the navi
-- switched, before any battle; a round's setup runs it on the navi's fresh stats, so a battle starts as the save's
-- reload left them.
function levels.reload(stats: Navi, level: number)
    local own = stats.navi.levels
    if own == nil then
        return
    end
    local base = own.base_hp[levels.CLEARED + 1]
    if base == nil then
        return
    end
    stats.max_hp = base
    stats.max_base_hp = base
    levels.add(stats, level)
    stats.hp = stats.max_hp
end

return levels
