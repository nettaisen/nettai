--!strict
-- The test content's own sprites, by name, each defined here with its made-up timing (sprite_timing.luau): the
-- navis', and one for every effect, every hit spark and every look the rules' roles name.

local timing = require("./sprite_timing")

local function sprite(name: string): SpriteAsset
    return asset.sprite(name, timing[name] or {})
end

return {
    ["test-megaman"] = sprite("test-megaman"),
    ["test-navi"] = sprite("test-navi"),
    ["test-burst"] = sprite("test-burst"),
    ["test-spark"] = sprite("test-spark"),
    ["test-sprite-charge-glow"] = sprite("test-sprite-charge-glow"),
    ["test-sprite-charge-glow-a"] = sprite("test-sprite-charge-glow-a"),
    ["test-sprite-full-synchro-aura"] = sprite("test-sprite-full-synchro-aura"),
    ["test-sprite-confusion"] = sprite("test-sprite-confusion"),
    ["test-sprite-blindness"] = sprite("test-sprite-blindness"),
    ["test-sprite-immobilized"] = sprite("test-sprite-immobilized"),
    ["test-sprite-ice"] = sprite("test-sprite-ice"),
    ["test-sprite-bubble"] = sprite("test-sprite-bubble"),
    ["test-sprite-hit-marker"] = sprite("test-sprite-hit-marker"),
    ["test-sprite-lockon-marker"] = sprite("test-sprite-lockon-marker"),
}
