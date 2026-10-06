--!strict
-- EXE5's API (docs/design/rules-in-luau.md, As built S8, as EXE6's rules/api.luau): what EXE5's chips, navis and
-- libraries ask of EXE5's rules, which keep it in their state and setup. Content outside rules/ asks it here, never
-- the rules' state or setup itself.

local exe5 = {}

-- The side's level (0 to its team navi's story's last), or nil (MegaMan, who takes none): the rules' `level`
-- (rules/save), the count of the save's story flags the init exchange sends. The engine knows nothing of it: the team
-- navis' damage rows read it (lib/navi_level).
function exe5.navi_level(side: number): number?
    local s = rules.setup_of(side) :: { level: number? }?
    return if s then s.level else nil
end

return exe5
