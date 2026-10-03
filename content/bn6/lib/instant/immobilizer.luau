--!strict
-- BN6's immobilizing hit (the original's attack object #0x3F, `sub_80CC044`;
-- docs/engine/dimming-chips.md): the shared immobilizer
-- (@common/instant/immobilizer), BN6's kind: unseen, a guard-breaking attack
-- against navis that immobilizes (status 0x42). Instant chip effect 9
-- spawns it (immobilize.luau).

local collision = require("../../rules/collision")
local common = require("@common/instant/immobilizer")
local statuses = require("../../rules/status")

local immobilizer = {}

immobilizer.kind = common.make_kind("bn6:immobilizer", {
    sound = asset.sound("bn6:immobilizer"),
    status = statuses.immobilize_120,
    target = collision.hits_navis,
    self_type = collision.guard_breaking,
})

-- `sub_80CC0B2` with `sub_801155A`: one on (x, y) for `owner`, no element
-- and no damage.
function immobilizer.spawn(owner: Object, x: number, y: number): Object?
    return common.spawn(immobilizer.kind, owner, x, y)
end

return immobilizer
