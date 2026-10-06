--!strict
-- A section of a game's root (docs/design/content-model-v2.md §4.0), as a
-- section's init returns it: the tables its modules return, by id, as one
-- (`merge { require("@self/airshot"), require("@self/cannon"), ... }`, in
-- their paths' order). An id two modules give is an error naming it.

return function(parts: { {} }): { [string]: any }
    local out = {}
    for _, part in parts do
        -- Modules export records with named fields, rather than string indexers.
        for id, definition in part :: { [string]: any } do
            if out[id] ~= nil then
                error(string.format("two modules give %s", tostring(id)), 2)
            end
            out[id] = definition
        end
    end
    return out
end
