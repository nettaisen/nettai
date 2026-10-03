--!strict
-- The engine's test lock-on modes, under the BN6 modes' names (the BN6
-- overlay's chips name them): made-up modes for the tests. `cannon` is next
-- to the target with the column shifts; `thunder` beside the target or
-- diagonally behind it; `beast_lunge` a panel or two in front of it (only
-- two away in the far column); `beast_claw` next to it; the rest stay.
-- (The column shifts and the clear-path condition are the test content's
-- rules.)

local function stay(id: string): LockonDef
    return define.lockon { id = id, rule = "stay" }
end

return {
    stay = stay("test:stay"),
    cannon = define.lockon { id = "test:cannon", rule = "near", offsets = { { -1, 0 } }, column_shifts = true },
    thunder = define.lockon { id = "test:thunder", rule = "near", offsets = { { -1, 0 }, { -1, 1 } } },
    widesht = stay("test:widesht"),
    firebrn1 = stay("test:firebrn1"),
    yoyo = stay("test:yoyo"),
    drksword = stay("test:drksword"),
    crakshot = stay("test:crakshot"),
    dublshot = stay("test:dublshot"),
    gundels1 = stay("test:gundels1"),
    trnarrw1 = stay("test:trnarrw1"),
    beast_lunge = define.lockon {
        id = "test:beast-lunge",
        rule = "near",
        offsets = { { -1, 0 }, { -2, 0 } },
        far_column_offsets = { { -2, 0 } },
    },
    beast_claw = define.lockon { id = "test:beast-claw", rule = "near", offsets = { { -1, 0 } } },
    crosswrd = stay("test:crosswrd"),
    sprsonic = stay("test:sprsonic"),
    bigbomb = stay("test:bigbomb"),
    gundelex = stay("test:gundelex"),
    elcpuls1 = stay("test:elcpuls1"),
    moonbld = stay("test:moonbld"),
}
