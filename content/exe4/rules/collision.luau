--!strict
-- Collision types (docs/engine/field-collision-damage.md §3.3): what an object is (its self type) and what it
-- reaches (its target type), a flag word for each side, from EXE4's table (0x08012660: 61 rows, EXE5's first 61
-- the same words), EXE5's names by row (content/exe5/rules/collision.luau). `row_offset` as EXE5's.
-- Written by tools/exe4/gen_rules.py (bn6battle-verify) from the EXE4 ROM; people own it now.

local collision = {}

collision.attack = new.collision { side0 = 0x80000000, side1 = 0x40000000, row_offset = 0x20 }
collision.thrown = new.collision { side0 = 0x80000008, side1 = 0x40000008, row_offset = 0x50 }
collision.slash = new.collision { side0 = 0x80002000, side1 = 0x40002000, row_offset = 0x38 }
collision.thrown_slash = new.collision { side0 = 0x80002008, side1 = 0x40002008, row_offset = 0xe0 }
collision.piercing = new.collision { side0 = 0x80000004, side1 = 0x40000004, row_offset = 0x58 }
collision.hits_navis = new.collision { side0 = 0x15800000, side1 = 0x2a800000, row_offset = 0x28 }
collision.hit_by_other_side = new.collision { side0 = 0x55800000, side1 = 0xaa800000, row_offset = 0xa0 }
collision.hit_by_other_side_or_blockers = new.collision { side0 = 0x55840000, side1 = 0xaa840000, row_offset = 0x68 }
collision.piercing_break = new.collision { side0 = 0x80000006, side1 = 0x40000006, row_offset = 0xb0 }
collision.own_object = new.collision { side0 = 0x20010000, side1 = 0x10010000, row_offset = 0x60 }
collision.everything = new.collision { side0 = 0xff800002, side1 = 0xff800002, row_offset = 0x78 }
collision.hit_by_other_attacks = new.collision { side0 = 0x50000000, side1 = 0xa0000000, row_offset = 0xc0 }
collision.mine = new.collision { side0 = 0xc004000e, side1 = 0xc004000e, row_offset = 0x198 }
collision.hits_both_sides = new.collision { side0 = 0x3d800000, side1 = 0x3e800000, row_offset = 0x150 }
collision.navi = new.collision { side0 = 0x08410000, side1 = 0x04210000, row_offset = 0x8 }
collision.navi_target = new.collision { side0 = 0x55800200, side1 = 0xaa800200, row_offset = 0x10 }
collision.floating_navi = new.collision { side0 = 0x08510000, side1 = 0x04310000, row_offset = 0x80 }
collision.thrown_break = new.collision { side0 = 0x8000000a, side1 = 0x4000000a, row_offset = 0xa8 }
collision.thrown_piercing_break = new.collision { side0 = 0x8000000e, side1 = 0x4000000e, row_offset = 0xb8 }
collision.pull = new.collision { side0 = 0x00004200, side1 = 0x00004200, row_offset = 0xf0 }
collision.breaking = new.collision { side0 = 0x80000002, side1 = 0x40000002, row_offset = 0x30 }
collision.chip_destruction = new.collision { side0 = 0x80000010, side1 = 0x40000010, row_offset = 0x40 }
collision.pushing = new.collision { side0 = 0x80000800, side1 = 0x40000800, row_offset = 0x90 }
collision.curse = new.collision { side0 = 0x8000000c, side1 = 0x4000000c, row_offset = 0xc8 }
collision.attack_480 = new.collision { side0 = 0x80000400, side1 = 0x40000400, row_offset = 0x128 }
collision.neutral_body = new.collision { side0 = 0x00810002, side1 = 0x00810002, row_offset = 0x70 }
collision.nothing = new.collision { side0 = 0x00000000, side1 = 0x00000000, row_offset = 0x0 }
collision.own_body = new.collision { side0 = 0x02010000, side1 = 0x01010000, row_offset = 0x98 }
collision.guard_breaking = new.collision { side0 = 0x80005000, side1 = 0x40005000, row_offset = 0x190 }
collision.drain = new.collision { side0 = 0x8000400c, side1 = 0x4000400c, row_offset = 0x160 }
collision.breaking_drain = new.collision { side0 = 0x8000400e, side1 = 0x4000400e, row_offset = 0x180 }
collision.collision_03 = new.collision { side0 = 0x51800100, side1 = 0xa2800100, row_offset = 0x18 }
collision.collision_09 = new.collision { side0 = 0x80000100, side1 = 0x40000100, row_offset = 0x48 }
collision.collision_11 = new.collision { side0 = 0x80002400, side1 = 0x40002400, row_offset = 0x88 }
collision.collision_1a = new.collision { side0 = 0x8000001e, side1 = 0x4000001e, row_offset = 0xd0 }
collision.collision_1b = new.collision { side0 = 0x80004020, side1 = 0x40004020, row_offset = 0xd8 }
collision.collision_1d = new.collision { side0 = 0x8000001c, side1 = 0x4000001c, row_offset = 0xe8 }
collision.collision_1f = new.collision { side0 = 0x80040002, side1 = 0x40040002, row_offset = 0xf8 }
collision.collision_20 = new.collision { side0 = 0x80000808, side1 = 0x40000808, row_offset = 0x100 }
collision.collision_21 = new.collision { side0 = 0x8004000a, side1 = 0x4004000a, row_offset = 0x108 }
collision.collision_22 = new.collision { side0 = 0xc004800e, side1 = 0xc004800e, row_offset = 0x110 }
collision.collision_23 = new.collision { side0 = 0x00080000, side1 = 0x00080000, row_offset = 0x118 }
collision.collision_24 = new.collision { side0 = 0x80000c08, side1 = 0x40000c08, row_offset = 0x120 }
collision.collision_26 = new.collision { side0 = 0xc0040008, side1 = 0xc0040008, row_offset = 0x130 }
collision.collision_27 = new.collision { side0 = 0xc0048002, side1 = 0xc0048002, row_offset = 0x138 }
collision.collision_28 = new.collision { side0 = 0x80002006, side1 = 0x40002006, row_offset = 0x140 }
collision.collision_29 = new.collision { side0 = 0x02010002, side1 = 0x01010002, row_offset = 0x148 }
collision.collision_2d = new.collision { side0 = 0x8000400a, side1 = 0x4000400a, row_offset = 0x168 }
collision.collision_2e = new.collision { side0 = 0x80004006, side1 = 0x40004006, row_offset = 0x170 }
collision.collision_2f = new.collision { side0 = 0x80004008, side1 = 0x40004008, row_offset = 0x178 }
collision.collision_31 = new.collision { side0 = 0x80005004, side1 = 0x40005004, row_offset = 0x188 }
collision.collision_35 = new.collision { side0 = 0x08410002, side1 = 0x04210002, row_offset = 0x1a8 }
collision.collision_36 = new.collision { side0 = 0x80004002, side1 = 0x40004002, row_offset = 0x1b0 }
collision.collision_37 = new.collision { side0 = 0x0201000a, side1 = 0x0101000a, row_offset = 0x1b8 }
collision.collision_38 = new.collision { side0 = 0x0f800000, side1 = 0x0f800000, row_offset = 0x1c0 }
collision.collision_39 = new.collision { side0 = 0x08510002, side1 = 0x04310002, row_offset = 0x1c8 }
collision.collision_3a = new.collision { side0 = 0x0841000a, side1 = 0x0421000a, row_offset = 0x1d0 }
collision.collision_3b = new.collision { side0 = 0x20010002, side1 = 0x10010002, row_offset = 0x1d8 }
collision.collision_3c = new.collision { side0 = 0x8000400a, side1 = 0x4000400a, row_offset = 0x1e0 }
collision.collision_2b = new.collision { side0 = 0x80000408, side1 = 0x40000408, row_offset = 0x158 }
collision.collision_34 = new.collision { side0 = 0x80004004, side1 = 0x40004004, row_offset = 0x1a0 }

return collision
