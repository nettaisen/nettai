--!strict
-- A soul's Chaos Unison weapon (AIData +0x11's routine, 0x30 to 0x3A): a dark chip loaded as the attack (0x0800FE78,
-- EXE6's `sub_8010D04`'s place: the chip's record's damage, hit parameter, lockout and element) and the chip's use.
-- An armed full B charge released in the chaos cycle's window fires it (rules/souls); the charge's time is the
-- routine's row of the charge table (0x0801CA62) by the chaos level, a level past 4 reading on into the next row's
-- first entry. ProtoSoul's is its own (routine 6, forms/protosoul/chaos).

local chaos = {}

-- The weapon loading dark chip `chip`, charging for `ticks` by the chaos level.
function chaos.weapon(chip: Chip, ticks: { number }): Weapon
    return new.weapon {
        charge_ticks = ticks,
        setup = function(navi: Object): Chip
            navi:load_chip_attack(chip)
            return chip
        end,
    }
end

return chaos
