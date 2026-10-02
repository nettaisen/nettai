--!strict
-- Chips the original has a record of and no routine for: the table their
-- use goes through has a null entry for them, and the game jumps to
-- address 0. (Gregar's and Falzar's dimming chip effects 34 and 35,
-- `off_802CCB4`. HackJack's and Django's navis, `off_802CD5C[18]` and
-- `[19]`, are null too; their chips use the Japanese games' navis,
-- chips/hackjack and chips/django.) A chip needs a use to be a chip, so
-- these are theirs: using one stops the battle with what the original
-- would have done.

local unusable = {}

local function stop(what: string): never
    error(what .. " has no routine in the original: its table entry is null, and the game jumps to address 0")
end

-- A `dimming` hook.
function unusable.dimming(what: string): (user: Object, spec: DimmingChipSpec) -> Object?
    return function(_user: Object, _spec: DimmingChipSpec): Object?
        return stop(what)
    end
end

return unusable
