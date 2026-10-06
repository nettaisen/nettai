--!strict
-- A game's rules made of its parts (docs/design/rules-in-luau.md §2.2): a game's rules are one definition
-- (`define.rules`), and its parts (the save, the NaviCust, the patch cards, the Crosses, Beast Out, the souls, ...)
-- are plain modules, each a table of what it adds: fields of the rules' state of a side and of a player's setup,
-- hooks, custom-screen hooks, buttons and windows, actions, extensions of the game's definitions. `compose.rules`
-- makes the one definition's table of them, beside the game's rule sections and roles: the parts' fields in one
-- state and one setup (a setup field two parts declare is one fact, of one type), and each hook one function that
-- calls the parts' in the order the list gives them, as the hook means:
--
-- - most hooks tell every part, in order (`round_setup`, `navi_intake`, `custom.open`, ...);
-- - a hook a part may answer is the first answer, in order (`chip_check`, `controller`, `starting_mood`,
--   `custom.hand_size`, ...): nil lets the next part answer;
-- - a hook a part answers true is the first true (`navi_bug`, `custom.keys`, `custom.take_back`).
--
-- A hook only one part has is that part's function itself.

local compose = {}

-- The hooks a part may answer: the first answer (not nil) is the hook's.
local FIRST_ANSWER = {
    chip_check = true,
    chip_cost = true,
    chip_substitute = true,
    controller = true,
    takeover = true,
    starting_mood = true,
    navi_palette = true,
    hand_size = true,
}

-- The hooks a part answers true: the first true is the hook's.
local FIRST_TRUE = {
    navi_bug = true,
    keys = true,
    take_back = true,
}

-- One function for a hook `name` the parts' `fns` (in order) have.
local function hook(name: string, fns: { (...any) -> any }): any
    if #fns == 1 then
        return fns[1]
    end
    if FIRST_ANSWER[name] then
        return function(...: any): any
            for _, f in fns do
                local answer = f(...)
                if answer ~= nil then
                    return answer
                end
            end
            return nil
        end
    elseif FIRST_TRUE[name] then
        return function(...: any): boolean
            for _, f in fns do
                if f(...) == true then
                    return true
                end
            end
            return false
        end
    end
    return function(...: any)
        for _, f in fns do
            f(...)
        end
    end
end

-- Whether two field types are the same (a type name, or a list of variant names).
local function same_type(a: any, b: any): boolean
    if type(a) ~= type(b) then
        return false
    end
    if type(a) ~= "table" then
        return a == b
    end
    if #a ~= #b then
        return false
    end
    for i, v in a do
        if b[i] ~= v then
            return false
        end
    end
    return true
end

-- The hooks of a table of them (a part's `hooks` or `custom`), in the parts' order, into `into`.
local function gather(into: { [string]: { (...any) -> any } }, names: { string }, hooks: { [string]: any }?)
    if hooks == nil then
        return
    end
    -- (In name order, so the merged table is the same whatever order the part's table holds them in.)
    local own: { string } = {}
    for name in hooks do
        table.insert(own, name)
    end
    table.sort(own)
    local all = hooks :: { [string]: any }
    for _, name in own do
        if into[name] == nil then
            into[name] = {}
            table.insert(names, name)
        end
        local fns = into[name] :: { any }
        table.insert(fns, all[name])
    end
end

-- The table `define.rules` takes: the parts' (in the order their hooks run) and `rest`'s (the game's rule sections
-- and roles).
function compose.rules(parts: { RulesPart }, rest: { [string]: any }): RulesSpec
    local spec: { [string]: any } = {}
    for k, v in rest do
        spec[k] = v
    end
    local state: { [string]: any } = {}
    local setup: { [string]: any } = {}
    local defaults: { [string]: any } = {}
    local buttons: { [string]: any } = {}
    local windows: { [string]: any } = {}
    local extends: { [string]: { [string]: any } } = {}
    local unplayable: { [string]: string } = {}
    local actions: { any } = {}
    local navi_state: any = nil
    local hooks: { [string]: { (...any) -> any } } = {}
    local hook_names: { string } = {}
    local custom: { [string]: { (...any) -> any } } = {}
    local custom_names: { string } = {}
    local function merge(into: { [string]: any }, from: { [string]: any }?, what: string)
        if from == nil then
            return
        end
        for k, v in from do
            if into[k] ~= nil then
                error(`rules: two parts give {what} \`{k}\``)
            end
            into[k] = v
        end
    end
    for i, each in parts do
        -- (A part's tables, read as tables of fields: their own types are the part's.)
        local part = each :: any
        merge(state, part.state, "the state's field")
        -- A setup field two parts declare is one fact: of one type.
        if part.setup ~= nil then
            for k, ty in part.setup do
                if setup[k] ~= nil and not same_type(setup[k], ty) then
                    error(`rules: two parts declare the setup's \`{k}\`, of other types`)
                end
                setup[k] = ty
            end
        end
        if part.setup_defaults ~= nil then
            for k, v in part.setup_defaults do
                if defaults[k] ~= nil and not same_type(defaults[k], v) then
                    error(`rules: two parts give the setup's \`{k}\` other defaults`)
                end
                defaults[k] = v
            end
        end
        merge(buttons, part.buttons, "a button")
        merge(windows, part.windows, "a window")
        merge(unplayable, part.unplayable_in_auto_battle, "a chip auto battle can't play,")
        if part.extends ~= nil then
            for registry, fields in part.extends do
                extends[registry] = extends[registry] or {}
                merge(extends[registry], fields, `an extension of {registry} definitions,`)
            end
        end
        if part.actions ~= nil then
            for _, a in part.actions do
                table.insert(actions, a)
            end
        end
        if part.navi_state ~= nil then
            if navi_state ~= nil then
                error(`rules: two parts give the state of the navis they drive (part {i})`)
            end
            navi_state = part.navi_state
        end
        gather(hooks, hook_names, part.hooks)
        gather(custom, custom_names, part.custom)
    end
    local function nonempty(t: { [string]: any }): { [string]: any }?
        return if next(t) ~= nil then t else nil
    end
    spec.state = nonempty(state)
    spec.setup = nonempty(setup)
    spec.setup_defaults = nonempty(defaults)
    spec.buttons = nonempty(buttons)
    spec.windows = nonempty(windows)
    spec.extends = nonempty(extends)
    spec.unplayable_in_auto_battle = nonempty(unplayable)
    spec.actions = if #actions > 0 then actions else nil
    spec.navi_state = navi_state
    local function hook_table(fns: { [string]: { (...any) -> any } }, names: { string }): { [string]: any }?
        if #names == 0 then
            return nil
        end
        local out: { [string]: any } = {}
        for _, name in names do
            out[name] = hook(name, fns[name])
        end
        return out
    end
    spec.hooks = hook_table(hooks, hook_names)
    spec.custom = hook_table(custom, custom_names)
    return spec :: any
end

return compose
