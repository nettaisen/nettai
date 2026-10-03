--!strict
-- BN5's bombs' parts, for BN6's throw (@bn6/lib/bombs/throw, the original's action 0x12; BN5's the same code but for
-- the thrown object's parameter, which BN5's passes none of): the bomb the navi holds (the attachment table's row
-- 4, 0c-02, BN5's sprite of BN6's look).

local attachment = require("@bn6/objects/attachment/attachment")

local bombs = {}

bombs.held = attachment.look { sprite = asset.sprite("bn5:bomb"), palette = 0 }

return bombs
