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

local WEAPONS: FormWeapons = { buster = buster.weapon, charge_shot = charged_shot.weapon }

-- `megaman`: the navi that changes form (it has forms, though this content
-- defines none but the base form).
local function navi(id: string, sprite: string, class: IdentityClass, ai_index: number, megaman: boolean): NaviDef
    return define.navi {
        id = id,
        sprite = asset.sprite(sprite),
        element = "null",
        buster_bonus = 1,
        move_lag = { 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4 },
        banners = { win = asset.banner("test-win"), lose = asset.banner("test-deleted") },
        merge_height = 0,
        run_message = {},
        weapons = WEAPONS,
        -- What a Cross change brings it with.
        fresh = { hp = 100, first_barrier = 0, mega_level = 5, giga_level = 1 },
        cross_hp = { 100, 150 },
        forms = if megaman then { gregar = { crosses = {} }, falzar = { crosses = {} } } else nil,
        identity = define.identity {
            class = class,
            version = 0,
            actor_type = "player",
            ai_index = ai_index,
            aura_anim = ai_index,
            ice = "medium",
            overlay_hooks = if megaman then { "death", "refresh", "flinch", "drag" } else nil,
            attach_points = attach_points(),
        },
    }
end

return {
    megaman = navi("megaman", "test-megaman", "megaman", 0, true),
    link_navi = navi("test/link-navi", "test-navi", "link_navi", 4, false),
    base = define.form {
        id = "base",
        kind = "base",
        sprite = asset.sprite("test-navi"),
        element = "null",
        buster_bonus = 0,
        weapons = WEAPONS,
        buster_arm = { anim = 0 },
    },
}
