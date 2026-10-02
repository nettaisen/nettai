--!strict
-- The engine's test statuses, under the BN6 statuses' names (the BN6
-- overlay's modules name them): each a made-up effect, no requests and 60
-- ticks on the paralysis timer. The freezing ones cancel the flinch and the
-- bubbling ones survive a counter hit, as BN6's do.

local PLAIN: { string } = {
    "test:paralyze-90", "test:paralyze-120", "test:paralyze-150", "test:paralyze-4", "test:paralyze-300", "test:paralyze-600",
    "test:confuse-480-past-paralyze", "test:confuse-720-past-paralyze", "test:confuse-960-past-paralyze",
    "test:confuse-4-past-paralyze", "test:confuse-300-past-paralyze", "test:confuse-600-past-paralyze",
    "test:blind-480-past-paralyze", "test:blind-720-past-paralyze", "test:blind-1200-past-paralyze",
    "test:blind-4-past-paralyze", "test:confuse-480", "test:confuse-720", "test:confuse-960", "test:confuse-4",
    "test:confuse-300", "test:confuse-600", "test:blind-480-past-confuse", "test:blind-720-past-confuse",
    "test:blind-1200-past-confuse", "test:blind-4-past-confuse", "test:blind-300-past-confuse",
    "test:blind-600-past-confuse", "test:immobilize-2-past-confuse", "test:immobilize-60-past-confuse",
    "test:immobilize-120-past-confuse", "test:immobilize-4-past-confuse", "test:blind-480", "test:blind-720",
    "test:blind-1200", "test:blind-4", "test:blind-300", "test:blind-600", "test:immobilize-2-past-blind",
    "test:immobilize-60-past-blind", "test:immobilize-120-past-blind", "test:immobilize-4-past-blind",
    "test:immobilize-300-past-blind", "test:immobilize-600-past-blind", "test:immobilize-30-past-blind",
    "test:freeze-150-past-blind", "test:freeze-150-2-past-blind", "test:freeze-150-3-past-blind", "test:immobilize-2",
    "test:immobilize-60", "test:immobilize-120", "test:immobilize-4", "test:immobilize-300", "test:immobilize-600",
    "test:immobilize-30", "test:freeze-150-past-immobilize", "test:freeze-150-2-past-immobilize",
    "test:freeze-150-3-past-immobilize", "test:freeze-4-past-immobilize", "test:freeze-300-past-immobilize",
    "test:freeze-600-past-immobilize", "test:bubble-150-past-immobilize", "test:bubble-150-2-past-immobilize",
    "test:bubble-150-3-past-immobilize", "test:bubble-150-past-freeze", "test:bubble-150-2-past-freeze",
    "test:bubble-150-3-past-freeze", "test:bubble-4-past-freeze", "test:bubble-300-past-freeze",
    "test:bubble-600-past-freeze", "test:after-table-1-past-freeze", "test:after-table-2-past-freeze", "test:after-table-3-past-freeze",
    "test:after-table-4-past-freeze", "test:after-table-1-past-bubble", "test:after-table-2-past-bubble", "test:after-table-3-past-bubble",
    "test:after-table-4-past-bubble", "test:after-table-5-past-bubble", "test:after-table-6-past-bubble", "test:after-table-7-past-bubble", "test:after-table-8-past-bubble",
    "test:after-table-9-past-bubble", "test:after-table-10-past-bubble",
}
local FREEZING: { string } = {
    "test:freeze-150", "test:freeze-150-2", "test:freeze-150-3", "test:freeze-4", "test:freeze-300", "test:freeze-600",
}
local BUBBLING: { string } = {
    "test:bubble-150", "test:bubble-150-2", "test:bubble-150-3", "test:bubble-4", "test:bubble-300", "test:bubble-600",
}

local status: { [string]: Status } = {}

local function define_all(keys: { string }, cancels_flinch: boolean, survives_counter: boolean)
    for _, key in keys do
        status[string.gsub(key, "-", "_")] = define.status {
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
