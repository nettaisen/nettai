--!strict
-- What the ruleset starts by role (docs/design/content-model-v2.md §7.4).

local antidmg_counter = require("../chips/antidmg/counter")
local antiswrd_counter = require("../chips/antiswrd/counter")
local barriers = require("../lib/barriers/barriers")
local bodygrd_counter = require("../chips/bodygrd/counter")
local charged_shot = require("../navis/00-megaman/weapons/charged-shot/weapon")
local encased_bubble = require("../objects/encased-bubble/bubble")

return define.roles {
    actions = {
        -- The charged shot a navi's request starts from idle without its
        -- weapon's setup (the original's action 0x16).
        forced_charged_shot = charged_shot.action,
        -- The trap chips' counters, which a trap that caught a hit starts
        -- (the original's actions 0x47, 0x48 and 0x4B; reactive.rs).
        anti_damage_counter = antidmg_counter.action,
        anti_sword_counter = antiswrd_counter.action,
        body_guard_counter = bodygrd_counter.action,
    },
    hooks = {
        -- The NaviCust FirstBarrier (`sub_8013892`): the navi enters with
        -- the barrier its stat names, which the game's program sets to type
        -- 1 (the Barrier chip's), and its visual.
        first_barrier = function(navi: Object)
            barriers.raise(navi, barriers.barrier_10, true)
        end,
        -- What an obstacle encased in ice or a bubble becomes
        -- (`sub_801813A`'s end).
        encased = encased_bubble.replace,
    },
}
