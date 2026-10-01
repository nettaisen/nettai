--!strict
-- The engine's test navis: MegaMan with his base form, and a link navi
-- with an actor record of its own (AI index 4), each with the BN6
-- overlay's buster and charged shot.

local buster = require("./megaman/weapons/buster/weapon")
local charged_shot = require("./megaman/weapons/charged-shot/weapon")

-- A sprite's attach points: the blade's point and the gun's (BN6's gun
-- sits at point 0x0D) ahead of the rest.
local function attach_points(): { { number } }
    local points = {}
    for i = 1, 34 do
        points[i] = { 4, 24 }
    end
    points[4] = { 20, 16 }
    points[0x0E] = { 20, 16 }
    return points
end

local WEAPONS = { buster = buster.weapon, charge_shot = charged_shot.weapon }

local function navi(id: string, name: string, sprite: string, class: IdentityClass, ai_index: number, number: number): NaviDef
    return define.navi {
        id = id,
        name = name,
        sprite = asset.sprite(sprite),
        element = "null",
        buster_bonus = 1,
        move_lag = { 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4 },
        banners = { win = asset.banner("test-win"), lose = asset.banner("test-deleted") },
        merge_height = 0,
        run_message = { 19, 12 },
        weapons = WEAPONS,
        -- What a Cross change brings it with.
        fresh = { hp = 100, first_barrier = 0, mega_level = 5, giga_level = 1 },
        cross_hp = { 100, 150 },
        identity = define.identity {
            class = class,
            version = 0,
            actor_type = "player",
            ai_index = ai_index,
            attach_points = attach_points(),
        },
        legacy = legacy { number = number, name_id = 0x1A0 + number },
    }
end

return {
    megaman = navi("megaman", "MegaMan", "test-megaman", "megaman", 0, 0),
    link_navi = navi("test/link-navi", "LinkNavi", "test-navi", "link_navi", 4, 1),
    base = define.form {
        id = "base",
        name = "Base",
        kind = "base",
        sprite = asset.sprite("test-navi"),
        element = "null",
        buster_bonus = 0,
        weapons = WEAPONS,
        legacy = legacy { number = 0 },
    },
}
