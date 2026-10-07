--!strict
-- EXE5's bullet (attack object 0x12, 0x080C6B98): the shared bullet (@exelib/bullet/bullet, EXE6's code) but for its
-- collision types. It probes as EXE5's row 0 (nothing; EXE6's row 0x3D) and bursts as EXE5's rows 4 and 5 (0x080C6BEC,
-- 0x080C6D10). Its burst sets no status (EXE6's `sub_80C6AB8` gives the variant's): EXE5's variants have none. EXE5 sets
-- the collision's element to the whole element byte where EXE6 splits it (no secondary elements: the same for EXE5's
-- elements). Its variants are EXE5's rows (0x080C6B20), each stated by its user (`bullet.variant`): the Vulcans',
-- SuprVulc's and the InfVulcs' (chips/vulcan, chips/infvulc), Spreader's (chips/spreader), the patch cards' HeatS's
-- and BubSht's (navis/megaman/weapons/spreads), NapalmSoul's charged shot's (forms/napalmsoul/charge), NapalmMan's
-- buster's (navis/napalmman/buster) and ColonelSoul's gun soldier's (objects/soldier).

local collision = require("../rules/collision")
local common = require("@exelib/bullet")

local bullet = {}

bullet.kind = common.make_kind("bullet", {
    probe = collision.nothing,
    attack = collision.attack,
    target = collision.hits_navis,
})

-- A bullet variant (a row of 0x080C6B20): its region, hit modifier, effect, sound and whether its effect shows once at
-- the bullet. Each user states its own.
bullet.variant = common.variant

return bullet
