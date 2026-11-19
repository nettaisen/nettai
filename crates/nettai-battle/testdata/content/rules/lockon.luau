--!strict
-- The engine's test lock-on modes, under the EXE6 modes' names (the EXE6
-- overlay's chips name them): made-up modes for the tests. `cannon` is next
-- to the target with the column shifts; `thunder` beside the target or
-- diagonally behind it; `beast_lunge` a panel or two in front of it (only
-- two away in the far column); `beast_claw` next to it; the rest stay.
-- `search`: the column shifts toward the user tried when no panel next to
-- the target fits, and what every panel between the chosen one and the
-- target must be for a clear path (free); `panel`, EXE6's search
-- (rules/lockon_search) with them.

local lockon_search = require("./lockon_search")

local LOCKON = schema.record_type("lockon")

local function stay(id: string): any
    return new.record(LOCKON, { id = id, rule = "stay" })
end

local lockon = {
    stay = stay("stay"),
    cannon = new.record(LOCKON, { id = "cannon", rule = "near", offsets = { { -1, 0 } }, column_shifts = true }),
    thunder = new.record(LOCKON, { id = "thunder", rule = "near", offsets = { { -1, 0 }, { -1, 1 } } }),
    wide_shot = stay("wide_shot"),
    fire_burner_1 = stay("fire_burner_1"),
    yo_yo = stay("yo_yo"),
    dark_sword = stay("dark_sword"),
    crack_shot = stay("crack_shot"),
    double_shot = stay("double_shot"),
    gun_del_sol_1 = stay("gun_del_sol_1"),
    train_arrow_1 = stay("train_arrow_1"),
    beast_lunge = new.record(LOCKON, {
        id = "beast_lunge",
        rule = "near",
        offsets = { { -1, 0 }, { -2, 0 } },
        far_column_offsets = { { -2, 0 } },
    }),
    beast_claw = new.record(LOCKON, { id = "beast_claw", rule = "near", offsets = { { -1, 0 } } }),
    cross_sword = stay("cross_sword"),
    super_sonic = stay("super_sonic"),
    big_bomb = stay("big_bomb"),
    gun_del_sol_ex = stay("gun_del_sol_ex"),
    elec_pulse_1 = stay("elec_pulse_1"),
    moon_blade = stay("moon_blade"),
    search = {
        column_shifts = { -1, -2 },
        clear_path = { occupied = false },
    },
}
lockon.LOCKON = LOCKON

function lockon.panel(navi: Object, x: number, y: number, mode: any): (number, number)
    return lockon_search.panel(navi, x, y, mode, lockon.search)
end

return lockon
