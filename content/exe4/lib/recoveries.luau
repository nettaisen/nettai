--!strict
-- EXE4's recoveries, the chips' action 0x1D (0x080EC71A), all in its first tick: the navi heals its variant's HP
-- (0x080EC74C: Recov10 to Recov300's 10, 30, 50, 80, 120, 150, 200 and 300, DrkRecov's 1000) through the heal
-- (0x0800ABCA: up to the maximum, with the recovery's sparkle and sound, `roles.effects.recovery` and
-- `sounds.recovery`), which an opponent's armed AntiRecv turns into damage; then, whether or not it did, the side's
-- mood gains the use's +0x0A (its record's counter byte, 0x0800F51E), the side's recoveries count one more
-- (0x08008034, statistic 5), and the attack ends. Each chip composes its own with its variant's HP.

local recoveries = {}

-- The side statistic counting recoveries.
local RECOVERIES = 5

export type Recovery = {
    id: string,
    hp: number,
}

function recoveries.action(spec: Recovery): Action
    local function update(me: Object, _s: any)
        me:heal(spec.hp, true)
        battle.gain_mood(me.alliance, me.hit_param)
        battle.bump_side_stat(me.alliance, RECOVERIES, 1)
        me:exit_attack()
    end
    return new.action { id = spec.id, update = update }
end

return recoveries
