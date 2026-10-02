--!strict
-- Chips the original has a record of and no routine for: the table their
-- use goes through has a null entry for them, and the game jumps to
-- address 0. (HackJack's and Django's navis 18 and 19, `off_802CD5C`;
-- Gregar's and Falzar's dimming chip effects 34 and 35, `off_802CCB4`, are
-- the Japanese ROMs': chips/gregar, chips/falzar.) A chip needs a use to
-- be a chip, so these are theirs: using one stops the battle with what the
-- original would have done.

local unusable = {}

local function stop(what: string): never
    error(what .. " has no routine in the original: its table entry is null, and the game jumps to address 0")
end

-- A `navi` hook.
function unusable.navi(what: string): (user: Object, controller: Object, spec: NaviChipSpec) -> Object?
    return function(_user: Object, _controller: Object, _spec: NaviChipSpec): Object?
        return stop(what)
    end
end

return unusable
