--!strict
-- What EXE4's twelve souls share (Double Soul, rules/souls): each a form of MegaMan's, named by its id, its number in
-- the original (NaviStats +0x24, 1 to 12) its place in his form list (his definition's `forms.souls`), as its chip's
-- (0x160 + the number): compat's (records.toml). A soul is the soul of the chips of its family given up for it
-- (0x08020008's table by soul); which souls a side has is its setup's (`souls`). Its chip, in its folder
-- (./<soul>/chip), is what OK puts in the hand for it, and the chip's use is the soul's change, its own
-- (rules/souls/change's maker). MegaMan's sprite in it is his entry plus the soul (0x0800B90A: 00-01 to 00-0c), his
-- face in the emotion window its own (0x08014B78: pictures 5 to 16), the navi he unites with comes in as its image
-- (@exelib/souls/image: its sprite, category 8 by the soul). He is taken for MegaMan in it (his identity: the chip
-- icons over him sit by his navi, NaviStats +0x23, 0x0800B9C0, whatever his soul). He changes by the souls' change and
-- revert (rules/souls). Its weapons are the soul's row of 0x080184F0 (AIData +0x0C to +0x0F, 0x0800BD48): the B
-- button's routine (+0x0D: the buster's, routine 0, but WindSoul's and MetalSoul's), the A charge's (+0x0E: FireSoul's,
-- ProtoSoul's, MetalSoul's and AquaSoul's) and the charged shot's (+0x0F), each soul's own (./<soul>/charge); its B+Left
-- special stays the base form's (AIData +0x00, which only ProtoSoul's status reset sets). What each soul does besides
-- (its status reset's routine, 0x0800E0A0 by soul) is still to port (docs/design/exe4-map.md §18 item 74).

local base = require("./base").base
local soul_image = require("../../../rules/souls/image")
local soul_revert = require("../../../rules/souls/revert")

local soul = {}

-- The revert out of every soul (rules/souls/revert), and the buster (weapon routine 0) the souls but WindSoul and
-- MetalSoul fire on B, his base form's.
soul.revert = soul_revert.action
soul.buster = base.weapons.buster
-- The weapons of a soul whose own are still to port: his base form's (§18 item 74).
soul.weapons = base.weapons

-- A soul's image at the change: its navi's sprite (rules/souls/image).
function soul.image(sprite: SpriteAsset): SoulImage
    return soul_image.look { sprite = sprite }
end

return soul
