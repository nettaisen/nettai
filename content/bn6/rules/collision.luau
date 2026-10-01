--!strict
-- Collision types (docs/engine/field-collision-damage.md §3.3): what an
-- object is (its self type) and what it reaches (its target type), as a
-- flag word for each side. An object's registration takes two
-- (`me:setup_collision(is, hits, hit_mod)`).
--
-- `row_offset` is the offset the original's lookup of the type's row
-- leaves in a register (its row times 8), which a damage word's bug code
-- takes as its high byte (`sub_801A00E`).
--
-- The types the converted content uses so far; the generator adds the
-- rest (docs/design/content-model-v2.md §9.3). Names other than `thrown`
-- and `hits-navis` (the design's) were made up by what the flags do; they
-- are listed there for review.

local collision = {}

-- An attack of its side that also reaches objects with status 0x20 (row
-- 0x04: a seed, a bomb's lingering hit, the bug and black bombs' bursts).
collision.attack = define.collision { id = "attack", side0 = 0x80000080, side1 = 0x40000080, row_offset = 0x20 }

-- A thrown attack (row 0x0A: a bomb, a FlashBomb in flight, the energy
-- burst's hits).
collision.thrown = define.collision { id = "thrown", side0 = 0x80000088, side1 = 0x40000088, row_offset = 0x50 }

-- A slash (row 0x07: the swords' hits).
collision.slash = define.collision { id = "slash", side0 = 0x80002080, side1 = 0x40002080, row_offset = 0x38 }

-- An attack that pierces invisibility and flashing (row 0x0B: the
-- FlashBomb's flash).
collision.piercing = define.collision { id = "piercing", side0 = 0x80000084, side1 = 0x40000084, row_offset = 0x58 }

-- What reaches the other side's objects, bodies and neutral objects: a
-- navi's attacks' target (row 0x05).
collision.hits_navis = define.collision { id = "hits-navis", side0 = 0x15800000, side1 = 0x2a800000, row_offset = 0x28 }

-- What reaches the other side's attacks, objects and bodies and neutral
-- objects (row 0x14: a BugBomb sitting); with the blocker bit, a
-- FlashBomb set down (row 0x0D).
collision.hit_by_other_side = define.collision {
    id = "hit-by-other-side",
    side0 = 0x55800000,
    side1 = 0xaa800000,
    row_offset = 0xA0,
}
collision.hit_by_other_side_or_blockers = define.collision {
    id = "hit-by-other-side-or-blockers",
    side0 = 0x55840000,
    side1 = 0xaa840000,
    row_offset = 0x68,
}

-- An object of its side, while dimmed too (row 0x0C: a FlashBomb set
-- down).
collision.own_object = define.collision { id = "own-object", side0 = 0x20010080, side1 = 0x10010080, row_offset = 0x60 }

-- Another body of its side that a thrown attack reaches, while dimmed too
-- (row 0x4E: a BugBomb sitting).
collision.own_thrown_body = define.collision {
    id = "own-thrown-body",
    side0 = 0x02010088,
    side1 = 0x01010088,
    row_offset = 0x270,
}

-- A neutral object, while dimmed too (row 0x4F: a BlkBomb).
collision.neutral_object = define.collision {
    id = "neutral-object",
    side0 = 0x0081008a,
    side1 = 0x0081008a,
    row_offset = 0x278,
}

-- What reaches everything: attacks, objects and bodies of both sides and
-- neutral objects (row 0x0F: what a BlkBomb reacts to).
collision.everything = define.collision { id = "everything", side0 = 0xff800002, side1 = 0xff800002, row_offset = 0x78 }

-- The navi chips' and link navis' types (group A2; named by what their
-- flags add to an attack's: 0x02 breaks, thawing a frozen target; 0x04
-- pierces invisibility and flashing; 0x08 as a thrown attack's).

-- A navi's body (row 0x01), and what a navi hits (row 0x02).
collision.navi = define.collision { id = "navi", side0 = 0x08410080, side1 = 0x04210080, row_offset = 0x08 }
collision.navi_target = define.collision { id = "navi-target", side0 = 0x55800200, side1 = 0xaa800200, row_offset = 0x10 }

-- A breaking attack (row 0x06: DustMan's junk, the link navis' strikes).
collision.breaking = define.collision { id = "breaking", side0 = 0x80000082, side1 = 0x40000082, row_offset = 0x30 }

-- A breaking thrown attack (row 0x15: GroundMan's drill and rocks).
collision.breaking_thrown = define.collision { id = "breaking-thrown", side0 = 0x8000008a, side1 = 0x4000008a, row_offset = 0xA8 }

-- A breaking attack that pierces (row 0x16: CircusMan's show).
collision.breaking_piercing = define.collision { id = "breaking-piercing", side0 = 0x80000086, side1 = 0x40000086, row_offset = 0xB0 }

-- A breaking thrown attack that pierces (row 0x17: Django's Gun del Sol).
collision.breaking_thrown_piercing = define.collision {
    id = "breaking-thrown-piercing",
    side0 = 0x8000008e,
    side1 = 0x4000008e,
    row_offset = 0xB8,
}

-- A thrown attack that pierces (row 0x19: JudgeMan's lash).
collision.thrown_piercing = define.collision { id = "thrown-piercing", side0 = 0x8000008c, side1 = 0x4000008c, row_offset = 0xC8 }

-- A pull of no side (row 0x1E: DustMan's DustBrk drawing things in).
collision.pull = define.collision { id = "pull", side0 = 0x00004280, side1 = 0x00004280, row_offset = 0xF0 }

return collision
