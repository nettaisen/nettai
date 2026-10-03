--!strict
-- Collision types (docs/engine/field-collision-damage.md §3.3): what an object is (its self type) and what it
-- reaches (its target type), a flag word for each side, from BN5's table (0x0801636C, BN6's `byte_8019C7C`: 80
-- rows to BN6's 89): BN6's types by their names and rows, with BN5's words, BN6's past the table's end left out.
-- BN5's are BN6's but for the 0x80 bit, which BN6 adds to the self type of every attack and object, and row 0x3D
-- (BN5 gives it its sides' bits: bn5-map.md §15). `row_offset` as BN6's.
-- Written by tools/bn5/gen_rules.py (bn6battle-verify) from the BN5 ROM; people own it now.

local collision = {}

-- Row 0x04 (BN6's 0x80000080, 0x40000080).
collision.attack = define.collision { id = "attack", side0 = 0x80000000, side1 = 0x40000000, row_offset = 0x20 }
-- Row 0x0a (BN6's 0x80000088, 0x40000088).
collision.thrown = define.collision { id = "thrown", side0 = 0x80000008, side1 = 0x40000008, row_offset = 0x50 }
-- Row 0x07 (BN6's 0x80002080, 0x40002080).
collision.slash = define.collision { id = "slash", side0 = 0x80002000, side1 = 0x40002000, row_offset = 0x38 }
-- Row 0x1c (BN6's 0x80002088, 0x40002088).
collision.thrown_slash = define.collision { id = "thrown-slash", side0 = 0x80002008, side1 = 0x40002008, row_offset = 0xe0 }
-- Row 0x0b (BN6's 0x80000084, 0x40000084).
collision.piercing = define.collision { id = "piercing", side0 = 0x80000004, side1 = 0x40000004, row_offset = 0x58 }
-- Row 0x05.
collision.hits_navis = define.collision { id = "hits-navis", side0 = 0x15800000, side1 = 0x2a800000, row_offset = 0x28 }
-- Row 0x14.
collision.hit_by_other_side = define.collision { id = "hit-by-other-side", side0 = 0x55800000, side1 = 0xaa800000, row_offset = 0xa0 }
-- Row 0x0d.
collision.hit_by_other_side_or_blockers = define.collision { id = "hit-by-other-side-or-blockers", side0 = 0x55840000, side1 = 0xaa840000, row_offset = 0x68 }
-- Row 0x16 (BN6's 0x80000086, 0x40000086).
collision.piercing_break = define.collision { id = "piercing-break", side0 = 0x80000006, side1 = 0x40000006, row_offset = 0xb0 }
-- Row 0x0c (BN6's 0x20010080, 0x10010080).
collision.own_object = define.collision { id = "own-object", side0 = 0x20010000, side1 = 0x10010000, row_offset = 0x60 }
-- Row 0x4e (BN6's 0x02010088, 0x01010088).
collision.own_thrown_body = define.collision { id = "own-thrown-body", side0 = 0x02010008, side1 = 0x01010008, row_offset = 0x270 }
-- Row 0x4f (BN6's 0x0081008a, 0x0081008a).
collision.neutral_object = define.collision { id = "neutral-object", side0 = 0x0081000a, side1 = 0x0081000a, row_offset = 0x278 }
-- Row 0x0f.
collision.everything = define.collision { id = "everything", side0 = 0xff800002, side1 = 0xff800002, row_offset = 0x78 }
-- Row 0x18.
collision.hit_by_other_attacks = define.collision { id = "hit-by-other-attacks", side0 = 0x50000000, side1 = 0xa0000000, row_offset = 0xc0 }
-- Row 0x33.
collision.mine = define.collision { id = "mine", side0 = 0xc004000e, side1 = 0xc004000e, row_offset = 0x198 }
-- Row 0x2a.
collision.hits_both_sides = define.collision { id = "hits-both-sides", side0 = 0x3d800000, side1 = 0x3e800000, row_offset = 0x150 }
-- Row 0x01 (BN6's 0x08410080, 0x04210080).
collision.navi = define.collision { id = "navi", side0 = 0x08410000, side1 = 0x04210000, row_offset = 0x8 }
-- Row 0x02.
collision.navi_target = define.collision { id = "navi-target", side0 = 0x55800200, side1 = 0xaa800200, row_offset = 0x10 }
-- Row 0x10 (BN6's 0x08510080, 0x04310080).
collision.floating_navi = define.collision { id = "floating-navi", side0 = 0x08510000, side1 = 0x04310000, row_offset = 0x80 }
-- Row 0x15 (BN6's 0x8000008a, 0x4000008a).
collision.thrown_break = define.collision { id = "thrown-break", side0 = 0x8000000a, side1 = 0x4000000a, row_offset = 0xa8 }
-- Row 0x17 (BN6's 0x8000008e, 0x4000008e).
collision.thrown_piercing_break = define.collision { id = "thrown-piercing-break", side0 = 0x8000000e, side1 = 0x4000000e, row_offset = 0xb8 }
-- Row 0x1e (BN6's 0x00004280, 0x00004280).
collision.pull = define.collision { id = "pull", side0 = 0x00004200, side1 = 0x00004200, row_offset = 0xf0 }
-- Row 0x06 (BN6's 0x80000082, 0x40000082).
collision.breaking = define.collision { id = "breaking", side0 = 0x80000002, side1 = 0x40000002, row_offset = 0x30 }
-- Row 0x08 (BN6's 0x80000090, 0x40000090).
collision.attack_90 = define.collision { id = "attack-90", side0 = 0x80000010, side1 = 0x40000010, row_offset = 0x40 }
-- Row 0x12 (BN6's 0x80000880, 0x40000880).
collision.pushing = define.collision { id = "pushing", side0 = 0x80000800, side1 = 0x40000800, row_offset = 0x90 }
-- Row 0x19 (BN6's 0x8000008c, 0x4000008c).
collision.curse = define.collision { id = "curse", side0 = 0x8000000c, side1 = 0x4000000c, row_offset = 0xc8 }
-- Row 0x25 (BN6's 0x80000480, 0x40000480).
collision.attack_480 = define.collision { id = "attack-480", side0 = 0x80000400, side1 = 0x40000400, row_offset = 0x128 }
-- Row 0x47 (BN6's 0x800000a0, 0x400000a0).
collision.attack_a0 = define.collision { id = "attack-a0", side0 = 0x80000020, side1 = 0x40000020, row_offset = 0x238 }
-- Row 0x0e (BN6's 0x00810082, 0x00810082).
collision.neutral_body = define.collision { id = "neutral-body", side0 = 0x00810002, side1 = 0x00810002, row_offset = 0x70 }
-- Row 0x3d (BN6's 0x00000080, 0x00000080).
collision.probe = define.collision { id = "probe", side0 = 0x80000080, side1 = 0x40000080, row_offset = 0x1e8 }
-- Row 0x00.
collision.nothing = define.collision { id = "nothing", side0 = 0x00000000, side1 = 0x00000000, row_offset = 0x0 }
-- Row 0x13 (BN6's 0x02010080, 0x01010080).
collision.own_body = define.collision { id = "own-body", side0 = 0x02010000, side1 = 0x01010000, row_offset = 0x98 }
-- Row 0x4a (BN6's 0x80000482, 0x40000482).
collision.second_drill = define.collision { id = "second-drill", side0 = 0x80000402, side1 = 0x40000402, row_offset = 0x250 }
-- Row 0x32 (BN6's 0x80005080, 0x40005080).
collision.guard_breaking = define.collision { id = "guard-breaking", side0 = 0x80005000, side1 = 0x40005000, row_offset = 0x190 }
-- Row 0x2c (BN6's 0x8000408c, 0x4000408c).
collision.drain = define.collision { id = "drain", side0 = 0x8000400c, side1 = 0x4000400c, row_offset = 0x160 }
-- Row 0x30 (BN6's 0x8000408e, 0x4000408e).
collision.breaking_drain = define.collision { id = "breaking-drain", side0 = 0x8000400e, side1 = 0x4000400e, row_offset = 0x180 }
-- Row 0x03.
collision.collision_03 = define.collision { id = "collision-03", side0 = 0x51800100, side1 = 0xa2800100, row_offset = 0x18 }
-- Row 0x09 (BN6's 0x80000180, 0x40000180).
collision.collision_09 = define.collision { id = "collision-09", side0 = 0x80000100, side1 = 0x40000100, row_offset = 0x48 }
-- Row 0x11 (BN6's 0x80002480, 0x40002480).
collision.collision_11 = define.collision { id = "collision-11", side0 = 0x80002400, side1 = 0x40002400, row_offset = 0x88 }
-- Row 0x1a (BN6's 0x8000009E, 0x4000009E).
collision.collision_1a = define.collision { id = "collision-1a", side0 = 0x8000001e, side1 = 0x4000001e, row_offset = 0xd0 }
-- Row 0x1b (BN6's 0x800040A0, 0x400040A0).
collision.collision_1b = define.collision { id = "collision-1b", side0 = 0x80004020, side1 = 0x40004020, row_offset = 0xd8 }
-- Row 0x1d (BN6's 0x8000009C, 0x4000009C).
collision.collision_1d = define.collision { id = "collision-1d", side0 = 0x8000001c, side1 = 0x4000001c, row_offset = 0xe8 }
-- Row 0x1f (BN6's 0x80040082, 0x40040082).
collision.collision_1f = define.collision { id = "collision-1f", side0 = 0x80040002, side1 = 0x40040002, row_offset = 0xf8 }
-- Row 0x20 (BN6's 0x80000888, 0x40000888).
collision.collision_20 = define.collision { id = "collision-20", side0 = 0x80000808, side1 = 0x40000808, row_offset = 0x100 }
-- Row 0x21 (BN6's 0x8004008A, 0x4004008A).
collision.collision_21 = define.collision { id = "collision-21", side0 = 0x8004000a, side1 = 0x4004000a, row_offset = 0x108 }
-- Row 0x22 (BN6's 0xC004808E, 0xC004808E).
collision.collision_22 = define.collision { id = "collision-22", side0 = 0xc004800e, side1 = 0xc004800e, row_offset = 0x110 }
-- Row 0x23.
collision.collision_23 = define.collision { id = "collision-23", side0 = 0x00080000, side1 = 0x00080000, row_offset = 0x118 }
-- Row 0x24 (BN6's 0x80000C88, 0x40000C88).
collision.collision_24 = define.collision { id = "collision-24", side0 = 0x80000c08, side1 = 0x40000c08, row_offset = 0x120 }
-- Row 0x26 (BN6's 0xC0040088, 0xC0040088).
collision.collision_26 = define.collision { id = "collision-26", side0 = 0xc0040008, side1 = 0xc0040008, row_offset = 0x130 }
-- Row 0x27 (BN6's 0xC0048082, 0xC0048082).
collision.collision_27 = define.collision { id = "collision-27", side0 = 0xc0048002, side1 = 0xc0048002, row_offset = 0x138 }
-- Row 0x28 (BN6's 0x80002086, 0x40002086).
collision.collision_28 = define.collision { id = "collision-28", side0 = 0x80002006, side1 = 0x40002006, row_offset = 0x140 }
-- Row 0x29 (BN6's 0x02010082, 0x01010082).
collision.collision_29 = define.collision { id = "collision-29", side0 = 0x02010002, side1 = 0x01010002, row_offset = 0x148 }
-- Row 0x2d (BN6's 0x8000408A, 0x4000408A).
collision.collision_2d = define.collision { id = "collision-2d", side0 = 0x8000400a, side1 = 0x4000400a, row_offset = 0x168 }
-- Row 0x2e (BN6's 0x80004086, 0x40004086).
collision.collision_2e = define.collision { id = "collision-2e", side0 = 0x80004006, side1 = 0x40004006, row_offset = 0x170 }
-- Row 0x2f (BN6's 0x80004088, 0x40004088).
collision.collision_2f = define.collision { id = "collision-2f", side0 = 0x80004008, side1 = 0x40004008, row_offset = 0x178 }
-- Row 0x31 (BN6's 0x80005084, 0x40005084).
collision.collision_31 = define.collision { id = "collision-31", side0 = 0x80005004, side1 = 0x40005004, row_offset = 0x188 }
-- Row 0x35 (BN6's 0x08410082, 0x04210082).
collision.collision_35 = define.collision { id = "collision-35", side0 = 0x08410002, side1 = 0x04210002, row_offset = 0x1a8 }
-- Row 0x36 (BN6's 0x80004082, 0x40004082).
collision.collision_36 = define.collision { id = "collision-36", side0 = 0x80004002, side1 = 0x40004002, row_offset = 0x1b0 }
-- Row 0x37 (BN6's 0x0201008A, 0x0101008A).
collision.collision_37 = define.collision { id = "collision-37", side0 = 0x0201000a, side1 = 0x0101000a, row_offset = 0x1b8 }
-- Row 0x38.
collision.collision_38 = define.collision { id = "collision-38", side0 = 0x0f800000, side1 = 0x0f800000, row_offset = 0x1c0 }
-- Row 0x39 (BN6's 0x08510082, 0x04310082).
collision.collision_39 = define.collision { id = "collision-39", side0 = 0x08510002, side1 = 0x04310002, row_offset = 0x1c8 }
-- Row 0x3a (BN6's 0x0841008A, 0x0421008A).
collision.collision_3a = define.collision { id = "collision-3a", side0 = 0x0841000a, side1 = 0x0421000a, row_offset = 0x1d0 }
-- Row 0x3b (BN6's 0x20010082, 0x10010082).
collision.collision_3b = define.collision { id = "collision-3b", side0 = 0x20010002, side1 = 0x10010002, row_offset = 0x1d8 }
-- Row 0x3c (BN6's 0x8000408A, 0x4000408A).
collision.collision_3c = define.collision { id = "collision-3c", side0 = 0x8000400a, side1 = 0x4000400a, row_offset = 0x1e0 }
-- Row 0x3e (BN6's 0x02110080, 0x01110080).
collision.collision_3e = define.collision { id = "collision-3e", side0 = 0x02110000, side1 = 0x01110000, row_offset = 0x1f0 }
-- Row 0x3f (BN6's 0x0851008A, 0x0431008A).
collision.collision_3f = define.collision { id = "collision-3f", side0 = 0x0851000a, side1 = 0x0431000a, row_offset = 0x1f8 }
-- Row 0x40 (BN6's 0x20012080, 0x10012080).
collision.collision_40 = define.collision { id = "collision-40", side0 = 0x20012000, side1 = 0x10012000, row_offset = 0x200 }
-- Row 0x41.
collision.collision_41 = define.collision { id = "collision-41", side0 = 0x04000000, side1 = 0x08000000, row_offset = 0x208 }
-- Row 0x42 (BN6's 0x80002090, 0x40002090).
collision.collision_42 = define.collision { id = "collision-42", side0 = 0x80002010, side1 = 0x40002010, row_offset = 0x210 }
-- Row 0x43 (BN6's 0x00040080, 0x00040080).
collision.collision_43 = define.collision { id = "collision-43", side0 = 0x00040000, side1 = 0x00040000, row_offset = 0x218 }
-- Row 0x44 (BN6's 0x20050080, 0x10050080).
collision.collision_44 = define.collision { id = "collision-44", side0 = 0x20050000, side1 = 0x10050000, row_offset = 0x220 }
-- Row 0x45 (BN6's 0x8000208A, 0x4000208A).
collision.collision_45 = define.collision { id = "collision-45", side0 = 0x8000200a, side1 = 0x4000200a, row_offset = 0x228 }
-- Row 0x46 (BN6's 0x800140A0, 0x400140A0).
collision.collision_46 = define.collision { id = "collision-46", side0 = 0x80014020, side1 = 0x40014020, row_offset = 0x230 }
-- Row 0x48 (BN6's 0xC0040080, 0xC0040080).
collision.hitbox = define.collision { id = "hitbox", side0 = 0xc0040000, side1 = 0xc0040000, row_offset = 0x240 }
-- Row 0x4b (BN6's 0x80000C88, 0x40000C88).
collision.collision_4b = define.collision { id = "collision-4b", side0 = 0x80000c08, side1 = 0x40000c08, row_offset = 0x258 }
-- Row 0x4c (BN6's 0x02014082, 0x01014082).
collision.collision_4c = define.collision { id = "collision-4c", side0 = 0x02014002, side1 = 0x01014002, row_offset = 0x260 }
-- Row 0x4d.
collision.collision_4d = define.collision { id = "collision-4d", side0 = 0x55000200, side1 = 0xaa000200, row_offset = 0x268 }
-- Row 0x2b: a 12-panel thunder ball's, DarkThnd's (0x080C9FA0) (BN6 names none).
collision.collision_2b = define.collision { id = "collision-2b", side0 = 0x80000408, side1 = 0x40000408, row_offset = 0x158 }

return collision
