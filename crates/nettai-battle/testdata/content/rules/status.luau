--!strict
-- The engine's test statuses, under the EXE6 statuses' names (the EXE6
-- overlay's modules name them): each a made-up effect, 60 ticks on the
-- paralysis timer (and its request). The freezing ones cancel the flinch and
-- the bubbling ones survive a counter hit, as EXE6's do.

local PLAIN: { string } = {
    "paralyze_90", "paralyze_120", "paralyze_150", "paralyze_4", "paralyze_300", "paralyze_600",
    "confuse_480_past_paralyze", "confuse_720_past_paralyze", "confuse_960_past_paralyze",
    "confuse_4_past_paralyze", "confuse_300_past_paralyze", "confuse_600_past_paralyze",
    "blind_480_past_paralyze", "blind_720_past_paralyze", "blind_1200_past_paralyze",
    "blind_4_past_paralyze", "confuse_480", "confuse_720", "confuse_960", "confuse_4",
    "confuse_300", "confuse_600", "blind_480_past_confuse", "blind_720_past_confuse",
    "blind_1200_past_confuse", "blind_4_past_confuse", "blind_300_past_confuse",
    "blind_600_past_confuse", "immobilize_2_past_confuse", "immobilize_60_past_confuse",
    "immobilize_120_past_confuse", "immobilize_4_past_confuse", "blind_480", "blind_720",
    "blind_1200", "blind_4", "blind_300", "blind_600", "immobilize_2_past_blind",
    "immobilize_60_past_blind", "immobilize_120_past_blind", "immobilize_4_past_blind",
    "immobilize_300_past_blind", "immobilize_600_past_blind", "immobilize_30_past_blind",
    "freeze_150_past_blind", "freeze_150_2_past_blind", "freeze_150_3_past_blind", "immobilize_2",
    "immobilize_60", "immobilize_120", "immobilize_4", "immobilize_300", "immobilize_600",
    "immobilize_30", "freeze_150_past_immobilize", "freeze_150_2_past_immobilize",
    "freeze_150_3_past_immobilize", "freeze_4_past_immobilize", "freeze_300_past_immobilize",
    "freeze_600_past_immobilize", "bubble_150_past_immobilize", "bubble_150_2_past_immobilize",
    "bubble_150_3_past_immobilize", "bubble_150_past_freeze", "bubble_150_2_past_freeze",
    "bubble_150_3_past_freeze", "bubble_4_past_freeze", "bubble_300_past_freeze",
    "bubble_600_past_freeze",
}
local FREEZING: { string } = {
    "freeze_150", "freeze_150_2", "freeze_150_3", "freeze_4", "freeze_300", "freeze_600",
}
local BUBBLING: { string } = {
    "bubble_150", "bubble_150_2", "bubble_150_3", "bubble_4", "bubble_300", "bubble_600",
}

local status: { [string]: Status } = {}

local function define_all(keys: { string }, cancels_flinch: boolean, survives_counter: boolean)
    for _, key in keys do
        -- (By the id: `paralyze_150` for `paralyze_150`.)
        local name = string.gsub(key, "-", "_")
        status[name] = new.status {
            id = key,
            duration = 60,
            timer = StatusKind.paralyze,
            cancels_flinch = cancels_flinch,
            survives_counter = survives_counter,
        }
    end
end

define_all(PLAIN, false, false)
define_all(FREEZING, true, false)
define_all(BUBBLING, false, true)

return status
