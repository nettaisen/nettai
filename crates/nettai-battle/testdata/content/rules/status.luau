--!strict
-- The engine's test statuses, under the BN6 statuses' names (the BN6
-- overlay's modules name them): each a made-up effect, no requests and 60
-- ticks on the paralysis timer. The freezing ones cancel the flinch and the
-- bubbling ones survive a counter hit, as BN6's do.

local PLAIN: { string } = {
    "paralyze-90", "paralyze-120", "paralyze-150", "paralyze-4", "paralyze-300", "paralyze-600",
    "confuse-480-past-paralyze", "confuse-720-past-paralyze", "confuse-960-past-paralyze",
    "confuse-4-past-paralyze", "confuse-300-past-paralyze", "confuse-600-past-paralyze",
    "blind-480-past-paralyze", "blind-720-past-paralyze", "blind-1200-past-paralyze",
    "blind-4-past-paralyze", "confuse-480", "confuse-720", "confuse-960", "confuse-4",
    "confuse-300", "confuse-600", "blind-480-past-confuse", "blind-720-past-confuse",
    "blind-1200-past-confuse", "blind-4-past-confuse", "blind-300-past-confuse",
    "blind-600-past-confuse", "immobilize-2-past-confuse", "immobilize-60-past-confuse",
    "immobilize-120-past-confuse", "immobilize-4-past-confuse", "blind-480", "blind-720",
    "blind-1200", "blind-4", "blind-300", "blind-600", "immobilize-2-past-blind",
    "immobilize-60-past-blind", "immobilize-120-past-blind", "immobilize-4-past-blind",
    "immobilize-300-past-blind", "immobilize-600-past-blind", "immobilize-30-past-blind",
    "freeze-150-past-blind", "freeze-150-2-past-blind", "freeze-150-3-past-blind", "immobilize-2",
    "immobilize-60", "immobilize-120", "immobilize-4", "immobilize-300", "immobilize-600",
    "immobilize-30", "freeze-150-past-immobilize", "freeze-150-2-past-immobilize",
    "freeze-150-3-past-immobilize", "freeze-4-past-immobilize", "freeze-300-past-immobilize",
    "freeze-600-past-immobilize", "bubble-150-past-immobilize", "bubble-150-2-past-immobilize",
    "bubble-150-3-past-immobilize", "bubble-150-past-freeze", "bubble-150-2-past-freeze",
    "bubble-150-3-past-freeze", "bubble-4-past-freeze", "bubble-300-past-freeze",
    "bubble-600-past-freeze", "after-table-1-past-freeze", "after-table-2-past-freeze", "after-table-3-past-freeze",
    "after-table-4-past-freeze", "after-table-1-past-bubble", "after-table-2-past-bubble", "after-table-3-past-bubble",
    "after-table-4-past-bubble", "after-table-5-past-bubble", "after-table-6-past-bubble", "after-table-7-past-bubble", "after-table-8-past-bubble",
    "after-table-9-past-bubble", "after-table-10-past-bubble",
}
local FREEZING: { string } = {
    "freeze-150", "freeze-150-2", "freeze-150-3", "freeze-4", "freeze-300", "freeze-600",
}
local BUBBLING: { string } = {
    "bubble-150", "bubble-150-2", "bubble-150-3", "bubble-4", "bubble-300", "bubble-600",
}

local status: { [string]: Status } = {}

local function define_all(keys: { string }, cancels_flinch: boolean, survives_counter: boolean)
    for _, key in keys do
        -- (By the id: `paralyze_150` for `paralyze-150`.)
        local name = string.gsub(key, "-", "_")
        status[name] = define.status {
            id = key,
            requests = 0,
            duration = 60,
            timer = "paralyze",
            cancels_flinch = cancels_flinch,
            survives_counter = survives_counter,
        }
    end
end

define_all(PLAIN, false, false)
define_all(FREEZING, true, false)
define_all(BUBBLING, false, true)

return status
