--!strict
-- A section of a game's root (docs/design/content-model-v2.md §4.0), as a
-- section's init returns it: the tables its modules return, by id, as one
-- (`merge { ["chips/cannon"] = require("@self/cannon"), ... }`, each by its
-- module's path in the pack). An id two modules give is an error naming
-- both.

return function(parts: { [string]: { [string]: any } }): { [string]: any }
    local modules = {}
    for module in parts do
        table.insert(modules, module)
    end
    table.sort(modules)
    local out, from = {}, {}
    for _, module in modules do
        for id, definition in parts[module] do
            if from[id] ~= nil then
                error(string.format("%s.luau and %s.luau both give %s", from[id], module, tostring(id)), 2)
            end
            out[id], from[id] = definition, module
        end
    end
    return out
end
