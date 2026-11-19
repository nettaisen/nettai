--!strict
-- The test content's own sprites, by name: the navis', and one for every effect, every hit spark and every look the
-- rules' roles name. (How a sprite animates is its sheet's, drawing's alone.)

local function sprite(name: string): SpriteAsset
    return asset.sprite(name)
end

return {
    test_megaman = sprite("test_megaman"),
    test_navi = sprite("test_navi"),
    test_burst = sprite("test_burst"),
    test_spark = sprite("test_spark"),
    test_sprite_charge_glow = sprite("test_sprite_charge_glow"),
    test_sprite_charge_glow_a = sprite("test_sprite_charge_glow_a"),
    test_sprite_full_synchro_aura = sprite("test_sprite_full_synchro_aura"),
    test_sprite_confusion = sprite("test_sprite_confusion"),
    test_sprite_blindness = sprite("test_sprite_blindness"),
    test_sprite_immobilized = sprite("test_sprite_immobilized"),
    test_sprite_ice = sprite("test_sprite_ice"),
    test_sprite_bubble = sprite("test_sprite_bubble"),
    test_sprite_hit_marker = sprite("test_sprite_hit_marker"),
    test_sprite_lockon_marker = sprite("test_sprite_lockon_marker"),
}
