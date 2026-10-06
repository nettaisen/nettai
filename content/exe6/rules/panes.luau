--!strict
-- What a tool shows of an EXE6 side's setup, pane by pane (the editor's: nettai-match's `panes`), beside the core's
-- own (the arena, the navi, the folder, the stats): each pane's title (a key of the locales' `[setup]`), whether a
-- side's shows it, and how it shows its facts. Whether a side's setup is right is ./init.luau's `validate`.

local board = require("./navicust/board")

-- Whether the side's navi changes form (MegaMan): the Crosses, the cards and the NaviCust are its.
local function changes_form(side: number): boolean
    local navi: NaviDef? = rules.setup_of(side).navi
    return navi ~= nil and navi.forms ~= nil
end

local panes: { PaneSpec } = {
    {
        title = "crosses",
        shown = changes_form,
        fields = {
            {
                field = "crosses",
                view = "list",
                checklist = true,
                -- The navi's Crosses, of both versions, Gregar's first.
                offered = function(side: number): { any }
                    local out = {}
                    local forms = (rules.setup_of(side).navi :: NaviDef).forms
                    if forms ~= nil then
                        for _, version in { "gregar", "falzar" } do
                            for _, f in (forms :: any)[version].crosses do
                                table.insert(out, f)
                            end
                        end
                    end
                    return out
                end,
            },
        },
    },
    {
        title = "patch_cards",
        shown = changes_form,
        fields = {
            {
                field = "patch_cards",
                view = "list",
                -- Their MB together, of the list's 80.
                summary = function(side: number): string
                    local mb = 0
                    for _, card in rules.setup_of(side).patch_cards do
                        mb += (card :: PatchCard).mb
                    end
                    return string.format("%d MB of 80", mb)
                end,
            },
        },
    },
    {
        title = "navicust",
        shown = changes_form,
        fields = {
            { field = "navicust_expansions", view = "pick", choices = { [0] = "4x4", [1] = "5x4", [2] = "5x5" } },
            {
                field = "navicust_programs",
                view = "grid",
                board = function(side: number): { string }?
                    local expansions: number? = rules.setup_of(side).navicust_expansions
                    return if expansions == nil then nil else board.boards[expansions + 1]
                end,
                rows_of_note = { [board.COMMAND_LINE + 1] = "command_line" },
                piece = "program",
                x = "x",
                y = "y",
                rotation = "rotation",
                color = "color",
                toggles = { "compressed" },
                shape = function(piece: any, compressed: boolean): { string }
                    local program: NaviCustProgram = piece
                    return if compressed and program.compressed ~= nil then program.compressed else program.shape
                end,
                colors = function(piece: any): { string }
                    local program: NaviCustProgram = piece
                    return program.colors
                end,
                palette = { white = 0xDEDEDE, pink = 0xF08CC8, yellow = 0xF0D840, red = 0xE85048, blue = 0x4C8CF0, green = 0x58C858 },
                badge = function(piece: any): string?
                    local program: NaviCustProgram = piece
                    return if program.plus then "+" else nil
                end,
            },
        },
    },
    {
        title = "sp_times",
        fields = {
            { field = "sp_times", view = "list", fixed = true, columns = { frames = { view = "number", format = "time" } } },
        },
    },
}

return panes
