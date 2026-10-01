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

-- An attack that pierces invisibility and flashing and breaks guards and
-- ice (row 0x16: the erase beams, Sensor's laser).
collision.piercing_break = define.collision {
    id = "piercing-break",
    side0 = 0x80000086,
    side1 = 0x40000086,
    row_offset = 0xB0,
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

-- A breaking attack (row 0x06: a shot that breaks guards; the break
-- family's shots, a thrown obstacle).
collision.breaking = define.collision { id = "breaking", side0 = 0x80000082, side1 = 0x40000082, row_offset = 0x30 }

-- An attack of row 0x08 (a projectile variant nothing names yet).
collision.attack_90 = define.collision { id = "attack-90", side0 = 0x80000090, side1 = 0x40000090, row_offset = 0x40 }

-- A pushing attack (row 0x12: AirShot's shot).
collision.pushing = define.collision { id = "pushing", side0 = 0x80000880, side1 = 0x40000880, row_offset = 0x90 }

-- An attack of row 0x17 (a flying shot variant nothing names yet).
collision.attack_8e = define.collision { id = "attack-8e", side0 = 0x8000008e, side1 = 0x4000008e, row_offset = 0xB8 }

-- The VDoll curse's hits (row 0x19).
collision.curse = define.collision { id = "curse", side0 = 0x8000008c, side1 = 0x4000008c, row_offset = 0xC8 }

-- An attack of row 0x25 (the wide shot's variant nothing fires).
collision.attack_480 = define.collision { id = "attack-480", side0 = 0x80000480, side1 = 0x40000480, row_offset = 0x128 }

-- An attack of row 0x47 (a projectile variant nothing names yet).
collision.attack_a0 = define.collision { id = "attack-a0", side0 = 0x800000a0, side1 = 0x400000a0, row_offset = 0x238 }

-- A neutral body, while dimmed too (row 0x0E: a VDoll that landed, a rock).
collision.neutral_body = define.collision { id = "neutral-body", side0 = 0x00810082, side1 = 0x00810082, row_offset = 0x70 }

-- A probe (row 0x3D): what finds the first thing its target type reaches
-- without hurting it (the bullet, the corn).
collision.probe = define.collision { id = "probe", side0 = 0x00000080, side1 = 0x00000080, row_offset = 0x1E8 }

-- Nothing (row 0x00: a target that reaches nothing; Discord's notes, which
-- confuse without learning they hit).
collision.nothing = define.collision { id = "nothing", side0 = 0x00000000, side1 = 0x00000000, row_offset = 0x00 }

-- Another body of its side, while dimmed too (row 0x13: the obstacles a
-- chip sets down: the instruments, AirRaid's plane, Sensor's turret, Wind's fan,
-- the Anubis statue).
collision.own_body = define.collision { id = "own-body", side0 = 0x02010080, side1 = 0x01010080, row_offset = 0x98 }

-- An attack that breaks guards (row 0x32: the instruments' notes).
collision.guard_breaking = define.collision {
    id = "guard-breaking",
    side0 = 0x80005080,
    side1 = 0x40005080,
    row_offset = 0x190,
}

-- A thrown attack that breaks (row 0x15: GolmHit's golem landing,
-- JustcOne's fist).
collision.thrown_break = define.collision { id = "thrown-break", side0 = 0x8000008a, side1 = 0x4000008a, row_offset = 0xA8 }

return collision
