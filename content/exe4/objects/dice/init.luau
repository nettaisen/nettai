--!strict
-- EXE4's die (attack object #5, 0x080CD9E8; its spawner 0x080CDCB8): EXE5's (@exelib/numbrman/dice: the same code)
-- with EXE4's differences: it states its own 40 HP and hit modifier 3 and no status (0x080CDA44, 0x080CDA74:
-- `dice.own`, which each thrower hands it), places itself on its panel in its init (0x080CDA4A), has no markers and no
-- face 9 of its own, and multiplies its damage's whole halfword by its face (0x080CDC6E); its effects are EXE4's rows
-- 0 (its explosion), 2 (off solid ground) and 0x15 (the battle's end), its sounds EXE4's (its bounce TopMan's spin's).
-- Its throwers: NumberMan (chips/numbrman: an attack at navis that runs while dimmed) and NumberSoul's charged shot
-- (action 0x15: a body the other side can hit).

local collision = require("../rules/collision")

local dice = {}

dice.made = require("@exelib/numbrman/dice").make {
    id = "dice",
    sprite = asset.sprite("dice"),
    explosion = new.effect { sprite = asset.sprite("explosion"), anim = 0, palette = 0 },
    puff = new.effect { sprite = asset.sprite("small-puff"), anim = 1, palette = 0 },
    gone = new.effect { sprite = asset.sprite("puff"), anim = 2, palette = 0 },
    blow_up = asset.sound("damage"),
    bounce = asset.sound("top-spin"),
    burst = asset.sound("heat-burst"),
    -- NumberMan's: an attack at navis (rows 4 and 5); the others': their own body, which the other side or a blocker
    -- hits (rows 12 and 13).
    attack = collision.attack,
    hits_navis = collision.hits_navis,
    body = collision.own_object,
    hit_by = collision.hit_by_other_side_or_blockers,
    places_itself = true,
}

-- What the die states of itself, whoever throws it.
dice.own = { hp = 40, status = nil :: Status?, hit_mod = 3 }

return dice
