--!strict
-- The engine's test lock-on modes, under the EXE6 modes' names (the EXE6
-- overlay's chips name them): made-up modes for the tests. `cannon` is next
-- to the target with the column shifts; `thunder` beside the target or
-- diagonally behind it; `beast_lunge` a panel or two in front of it (only
-- two away in the far column); `beast_claw` next to it; the rest stay.
-- `rules` is the rules' `lockon` section: the column shifts toward the
-- user tried when no panel next to the target fits, and what every panel
-- between the chosen one and the target must be for a clear path (free).

local function stay(id: string): LockonDef
    local mode: LockonDef = { id = id, rule = "stay" }
    return define.record("lockon", mode)
end

return {
    stay = stay("stay"),
    cannon = define.record("lockon", { id = "cannon", rule = "near", offsets = { { -1, 0 } }, column_shifts = true }),
    thunder = define.record("lockon", { id = "thunder", rule = "near", offsets = { { -1, 0 }, { -1, 1 } } }),
    widesht = stay("widesht"),
    firebrn1 = stay("firebrn1"),
    yoyo = stay("yoyo"),
    drksword = stay("drksword"),
    crakshot = stay("crakshot"),
    dublshot = stay("dublshot"),
    gundels1 = stay("gundels1"),
    trnarrw1 = stay("trnarrw1"),
    beast_lunge = define.record("lockon", {
        id = "beast-lunge",
        rule = "near",
        offsets = { { -1, 0 }, { -2, 0 } },
        far_column_offsets = { { -2, 0 } },
    }),
    beast_claw = define.record("lockon", { id = "beast-claw", rule = "near", offsets = { { -1, 0 } } }),
    crosswrd = stay("crosswrd"),
    sprsonic = stay("sprsonic"),
    bigbomb = stay("bigbomb"),
    gundelex = stay("gundelex"),
    elcpuls1 = stay("elcpuls1"),
    moonbld = stay("moonbld"),
    rules = {
        column_shifts = { -1, -2 },
        clear_path = {
            { require = 0x00000000, forbid = 0x0F880080 },
            { require = 0x00000000, forbid = 0x0F880080 },
        },
    },
}
