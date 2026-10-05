--!strict
-- EXE6's bombs' and seeds' throw (the original's action 0x12,
-- `sub_80EB628`): the shared throw (@exelib/bombs/throw) with EXE6's sound
-- and attachment. Each chip composes its own (`throw.action { held = ...,
-- thrower = ... }`): what it holds, and what throws what it throws
-- (docs/design/content-model-v2.md §5.1).

local attachment = require("../../objects/attachment")
local common = require("@exelib/bombs/throw")

local throw = {}

-- One state for every throw.
throw.STATE = common.STATE

-- Where a thrown thing leaves from.
throw.release_point = common.release_point

-- The damage word a thrown thing carries.
throw.damage_word = common.damage_word

local LOOK: ThrowLook = { sound = asset.sound("minibomb-throw"), attachment = attachment.kind }

-- A chip's throw.
function throw.action(spec: ThrowSpec): Action
    return common.action(spec, LOOK)
end

return throw
