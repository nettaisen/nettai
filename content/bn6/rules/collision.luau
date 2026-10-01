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

-- The rest of the original's rows (gen-content).
-- Row 0x01.
collision.player = define.collision { id = "player", side0 = 0x08410080, side1 = 0x04210080, row_offset = 0x8 }
-- Row 0x02.
collision.hits_player = define.collision { id = "hits-player", side0 = 0x55800200, side1 = 0xAA800200, row_offset = 0x10 }
-- Row 0x03.
collision.collision_03 = define.collision { id = "collision-03", side0 = 0x51800100, side1 = 0xA2800100, row_offset = 0x18 }
-- Row 0x06.
collision.collision_06 = define.collision { id = "collision-06", side0 = 0x80000082, side1 = 0x40000082, row_offset = 0x30 }
-- Row 0x08.
collision.collision_08 = define.collision { id = "collision-08", side0 = 0x80000090, side1 = 0x40000090, row_offset = 0x40 }
-- Row 0x09.
collision.collision_09 = define.collision { id = "collision-09", side0 = 0x80000180, side1 = 0x40000180, row_offset = 0x48 }
-- Row 0x10.
collision.collision_10 = define.collision { id = "collision-10", side0 = 0x08510080, side1 = 0x04310080, row_offset = 0x80 }
-- Row 0x11.
collision.collision_11 = define.collision { id = "collision-11", side0 = 0x80002480, side1 = 0x40002480, row_offset = 0x88 }
-- Row 0x12.
collision.collision_12 = define.collision { id = "collision-12", side0 = 0x80000880, side1 = 0x40000880, row_offset = 0x90 }
-- Row 0x15.
collision.collision_15 = define.collision { id = "collision-15", side0 = 0x8000008A, side1 = 0x4000008A, row_offset = 0xA8 }
-- Row 0x18.
collision.collision_18 = define.collision { id = "collision-18", side0 = 0x50000000, side1 = 0xA0000000, row_offset = 0xC0 }
-- Row 0x1A.
collision.collision_1a = define.collision { id = "collision-1a", side0 = 0x8000009E, side1 = 0x4000009E, row_offset = 0xD0 }
-- Row 0x1B.
collision.collision_1b = define.collision { id = "collision-1b", side0 = 0x800040A0, side1 = 0x400040A0, row_offset = 0xD8 }
-- Row 0x1C.
collision.collision_1c = define.collision { id = "collision-1c", side0 = 0x80002088, side1 = 0x40002088, row_offset = 0xE0 }
-- Row 0x1D.
collision.collision_1d = define.collision { id = "collision-1d", side0 = 0x8000009C, side1 = 0x4000009C, row_offset = 0xE8 }
-- Row 0x1E.
collision.collision_1e = define.collision { id = "collision-1e", side0 = 0x00004280, side1 = 0x00004280, row_offset = 0xF0 }
-- Row 0x1F.
collision.collision_1f = define.collision { id = "collision-1f", side0 = 0x80040082, side1 = 0x40040082, row_offset = 0xF8 }
-- Row 0x20.
collision.collision_20 = define.collision { id = "collision-20", side0 = 0x80000888, side1 = 0x40000888, row_offset = 0x100 }
-- Row 0x21.
collision.collision_21 = define.collision { id = "collision-21", side0 = 0x8004008A, side1 = 0x4004008A, row_offset = 0x108 }
-- Row 0x22.
collision.collision_22 = define.collision { id = "collision-22", side0 = 0xC004808E, side1 = 0xC004808E, row_offset = 0x110 }
-- Row 0x23.
collision.collision_23 = define.collision { id = "collision-23", side0 = 0x00080000, side1 = 0x00080000, row_offset = 0x118 }
-- Row 0x24.
collision.collision_24 = define.collision { id = "collision-24", side0 = 0x80000C88, side1 = 0x40000C88, row_offset = 0x120 }
-- Row 0x25.
collision.collision_25 = define.collision { id = "collision-25", side0 = 0x80000480, side1 = 0x40000480, row_offset = 0x128 }
-- Row 0x26.
collision.collision_26 = define.collision { id = "collision-26", side0 = 0xC0040088, side1 = 0xC0040088, row_offset = 0x130 }
-- Row 0x27.
collision.collision_27 = define.collision { id = "collision-27", side0 = 0xC0048082, side1 = 0xC0048082, row_offset = 0x138 }
-- Row 0x28.
collision.collision_28 = define.collision { id = "collision-28", side0 = 0x80002086, side1 = 0x40002086, row_offset = 0x140 }
-- Row 0x29.
collision.collision_29 = define.collision { id = "collision-29", side0 = 0x02010082, side1 = 0x01010082, row_offset = 0x148 }
-- Row 0x2A.
collision.eruption = define.collision { id = "eruption", side0 = 0x3D800000, side1 = 0x3E800000, row_offset = 0x150 }
-- Row 0x2C.
collision.drain = define.collision { id = "drain", side0 = 0x8000408C, side1 = 0x4000408C, row_offset = 0x160 }
-- Row 0x2D.
collision.collision_2d = define.collision { id = "collision-2d", side0 = 0x8000408A, side1 = 0x4000408A, row_offset = 0x168 }
-- Row 0x2E.
collision.collision_2e = define.collision { id = "collision-2e", side0 = 0x80004086, side1 = 0x40004086, row_offset = 0x170 }
-- Row 0x2F.
collision.collision_2f = define.collision { id = "collision-2f", side0 = 0x80004088, side1 = 0x40004088, row_offset = 0x178 }
-- Row 0x30.
collision.collision_30 = define.collision { id = "collision-30", side0 = 0x8000408E, side1 = 0x4000408E, row_offset = 0x180 }
-- Row 0x31.
collision.collision_31 = define.collision { id = "collision-31", side0 = 0x80005084, side1 = 0x40005084, row_offset = 0x188 }
-- Row 0x33.
collision.collision_33 = define.collision { id = "collision-33", side0 = 0xC004000E, side1 = 0xC004000E, row_offset = 0x198 }
-- Row 0x34.
collision.collision_34 = define.collision { id = "collision-34", side0 = 0x80004084, side1 = 0x40004084, row_offset = 0x1A0 }
-- Row 0x35.
collision.collision_35 = define.collision { id = "collision-35", side0 = 0x08410082, side1 = 0x04210082, row_offset = 0x1A8 }
-- Row 0x36.
collision.collision_36 = define.collision { id = "collision-36", side0 = 0x80004082, side1 = 0x40004082, row_offset = 0x1B0 }
-- Row 0x37.
collision.collision_37 = define.collision { id = "collision-37", side0 = 0x0201008A, side1 = 0x0101008A, row_offset = 0x1B8 }
-- Row 0x38.
collision.collision_38 = define.collision { id = "collision-38", side0 = 0x0F800000, side1 = 0x0F800000, row_offset = 0x1C0 }
-- Row 0x39.
collision.collision_39 = define.collision { id = "collision-39", side0 = 0x08510082, side1 = 0x04310082, row_offset = 0x1C8 }
-- Row 0x3A.
collision.collision_3a = define.collision { id = "collision-3a", side0 = 0x0841008A, side1 = 0x0421008A, row_offset = 0x1D0 }
-- Row 0x3B.
collision.collision_3b = define.collision { id = "collision-3b", side0 = 0x20010082, side1 = 0x10010082, row_offset = 0x1D8 }
-- Row 0x3C.
collision.collision_3c = define.collision { id = "collision-3c", side0 = 0x8000408A, side1 = 0x4000408A, row_offset = 0x1E0 }
-- Row 0x3D.
collision.collision_3d = define.collision { id = "collision-3d", side0 = 0x00000080, side1 = 0x00000080, row_offset = 0x1E8 }
-- Row 0x3E.
collision.collision_3e = define.collision { id = "collision-3e", side0 = 0x02110080, side1 = 0x01110080, row_offset = 0x1F0 }
-- Row 0x3F.
collision.collision_3f = define.collision { id = "collision-3f", side0 = 0x0851008A, side1 = 0x0431008A, row_offset = 0x1F8 }
-- Row 0x40.
collision.collision_40 = define.collision { id = "collision-40", side0 = 0x20012080, side1 = 0x10012080, row_offset = 0x200 }
-- Row 0x41.
collision.collision_41 = define.collision { id = "collision-41", side0 = 0x04000000, side1 = 0x08000000, row_offset = 0x208 }
-- Row 0x42.
collision.collision_42 = define.collision { id = "collision-42", side0 = 0x80002090, side1 = 0x40002090, row_offset = 0x210 }
-- Row 0x43.
collision.collision_43 = define.collision { id = "collision-43", side0 = 0x00040080, side1 = 0x00040080, row_offset = 0x218 }
-- Row 0x44.
collision.collision_44 = define.collision { id = "collision-44", side0 = 0x20050080, side1 = 0x10050080, row_offset = 0x220 }
-- Row 0x45.
collision.collision_45 = define.collision { id = "collision-45", side0 = 0x8000208A, side1 = 0x4000208A, row_offset = 0x228 }
-- Row 0x46.
collision.collision_46 = define.collision { id = "collision-46", side0 = 0x800140A0, side1 = 0x400140A0, row_offset = 0x230 }
-- Row 0x47.
collision.collision_47 = define.collision { id = "collision-47", side0 = 0x800000A0, side1 = 0x400000A0, row_offset = 0x238 }
-- Row 0x48.
collision.hitbox = define.collision { id = "hitbox", side0 = 0xC0040080, side1 = 0xC0040080, row_offset = 0x240 }
-- Row 0x4A.
collision.collision_4a = define.collision { id = "collision-4a", side0 = 0x80000482, side1 = 0x40000482, row_offset = 0x250 }
-- Row 0x4B.
collision.collision_4b = define.collision { id = "collision-4b", side0 = 0x80000C88, side1 = 0x40000C88, row_offset = 0x258 }
-- Row 0x4C.
collision.collision_4c = define.collision { id = "collision-4c", side0 = 0x02014082, side1 = 0x01014082, row_offset = 0x260 }
-- Row 0x4D.
collision.collision_4d = define.collision { id = "collision-4d", side0 = 0x55000200, side1 = 0xAA000200, row_offset = 0x268 }
-- Row 0x50.
collision.collision_50 = define.collision { id = "collision-50", side0 = 0x08412080, side1 = 0x04212080, row_offset = 0x280 }
-- Row 0x51.
collision.collision_51 = define.collision { id = "collision-51", side0 = 0x08410092, side1 = 0x04210092, row_offset = 0x288 }
-- Row 0x53.
collision.collision_53 = define.collision { id = "collision-53", side0 = 0x08412082, side1 = 0x04212082, row_offset = 0x298 }
-- Row 0x54.
collision.collision_54 = define.collision { id = "collision-54", side0 = 0x80002092, side1 = 0x40002092, row_offset = 0x2A0 }
-- Row 0x55.
collision.collision_55 = define.collision { id = "collision-55", side0 = 0x0841009A, side1 = 0x0421009A, row_offset = 0x2A8 }
-- Row 0x56.
collision.collision_56 = define.collision { id = "collision-56", side0 = 0x80000884, side1 = 0x40000884, row_offset = 0x2B0 }
-- Row 0x57.
collision.collision_57 = define.collision { id = "collision-57", side0 = 0x800000B2, side1 = 0x400000B2, row_offset = 0x2B8 }

return collision
