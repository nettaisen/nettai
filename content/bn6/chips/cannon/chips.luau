--!strict
-- Cannon, HiCannon and M-Cannon: a shot from the panel ahead that
-- staggers what it hits; the same cannon in three colours (the
-- original's action 0x14 with subtypes 0 to 2; lib/cannon.luau).
--
-- Their records stay the pack's for now (the GigaCans' Program Advance
-- recipes and the Cross special's chips name them by number): these are
-- their actions, which registration by number runs
-- (docs/design/content-model-v2.md §5.1, "As built").

local cannon = require("../../lib/cannon")
local variants = require("../../objects/projectile/variants")

return {
    cannon = cannon.action { id = "cannon/action", arm = cannon.arms.cannon, shot = variants.cannon },
    hicannon = cannon.action { id = "hicannon/action", arm = cannon.arms.hicannon, shot = variants.cannon },
    m_cannon = cannon.action { id = "m-cannon/action", arm = cannon.arms.m_cannon, shot = variants.cannon },
}
