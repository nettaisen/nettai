--!strict
-- What EXE4's twelve souls share (Double Soul, rules/souls): each a form of MegaMan's, named by its id, its number in
-- the original (NaviStats +0x24, 1 to 12) the place of its form in his lists (his definition's `forms`, by version), as
-- its chip's (0x160 + the number). A soul is the soul of the chips of its family given up for it (0x08020008's table
-- by soul), of one version (its flag, 0x08020018's table: Red Sun's souls 1 to 6, Blue Moon's 7 to 12). Its chip, in
-- its folder (./<soul>/chip), is what OK puts in the hand for it, and the chip's use is the soul's change, its own
-- (rules/souls/change's maker). MegaMan's sprite in it is his entry plus the soul (0x0800B90A: 00-01 to 00-0c), his
-- face in the emotion window its own (0x08014B78: pictures 5 to 16), the navi he unites with comes in as its image
-- (@exelib/souls/image: its sprite, category 8 by the soul). He is taken for MegaMan in it (his identity: the chip
-- icons over him sit by his navi, NaviStats +0x23, 0x0800B9C0, whatever his soul). He changes by the souls' change and
-- revert (rules/souls). What each soul does besides (its status reset's routine, 0x0800E0A0 by soul; its weapons) is still to
-- port (docs/design/exe4-map.md §18 item 74): in it he fights as his base form.

local base = require("./base").base
local soul_image = require("../../../rules/souls/image")
local soul_revert = require("../../../rules/souls/revert")

local soul = {}

-- The revert out of every soul (rules/souls/revert), and the weapons he fights with in it: his base form's, until the
-- souls' own (§18 item 74).
soul.revert = soul_revert.action
soul.weapons = base.weapons

-- A soul's image at the change: its navi's sprite (rules/souls/image).
function soul.image(sprite: SpriteAsset): SoulImage
    return soul_image.look { sprite = sprite }
end

return soul
