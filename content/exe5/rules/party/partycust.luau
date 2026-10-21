--!strict
-- PartyCust, the team navis' customizer (EXE5 DS: docs/design/exe5ds-map.md §4.10): each team navi has a gauge of
-- blocks (its definition's `party.gauge`), and a program bought as an item fills as many blocks as it costs. A
-- program's effects add to that navi's own stats record, whichever member of the party it fields: its maximum HP
-- (+0x5A, which 0x02004C84 adds to the base HP as the battle is set up), its buster's attack level (+0x01), the bonus
-- of its own chip's damage (+0x56, which 0x0203BF2C adds) and the support flag (+0x59: it may support and be
-- rescued with, 0x02004E9C). The customizer's apply switch (ov413 0x021E052C, by item − 0xB0) does exactly these
-- adds, without caps. A program is a definition of the game's `partycust_programs` (content/exe5/partycust/), made
-- its effects made by these; ./init.luau applies a partner's as the round is set up.

local partycust = {}

local function effect(e: PartyCustEffect): PartyCustEffect
    return e
end

-- P.HP+: the maximum HP's bonus (+0x5A, a halfword add).
function partycust.hp(n: number): PartyCustEffect
    return effect({ kind = "hp", amount = n })
end

-- P.Atk+: the buster's attack level (+0x01, a byte add that wraps).
function partycust.attack(n: number): PartyCustEffect
    return effect({ kind = "attack", amount = n })
end

-- P.Chp+: the bonus of the navi's own chip's damage (+0x56, a halfword add).
function partycust.chip_attack(n: number): PartyCustEffect
    return effect({ kind = "chip_attack", amount = n })
end

-- P.Spport: the navi may support (+0x59 = 1).
function partycust.support(): PartyCustEffect
    return effect({ kind = "support" })
end

-- What a navi's programs add to it: `stats` the member's block (`battle.member`), `base_hp` its base HP at the side's
-- level. The maximum HP is the base and the HP programs' sum (0x02004C84: +0x3E the base, +0x42 the base and the bonus),
-- and the navi enters at it (a save's team navi enters at its maximum, as a recording shows; the save keeps no other).
function partycust.apply(stats: Navi, base_hp: number, programs: { PartyCustProgram })
    local bonus = 0
    for _, program in programs do
        for _, e in program.effects do
            if e.kind == "hp" then
                bonus = int.u16(bonus + (e.amount :: number))
            elseif e.kind == "attack" then
                stats.attack = int.u8(stats.attack + (e.amount :: number))
            elseif e.kind == "chip_attack" then
                stats.chip_attack_bonus = int.u16(stats.chip_attack_bonus + (e.amount :: number))
            elseif e.kind == "support" then
                stats.supports_party = true
            end
        end
    end
    stats.max_base_hp = base_hp
    local hp = int.u16(base_hp + bonus)
    stats.max_hp = hp
    stats.hp = hp
end

-- The blocks `programs` fill.
function partycust.cost(programs: { PartyCustProgram }): number
    local n = 0
    for _, p in programs do
        n += p.cost
    end
    return n
end

return partycust
