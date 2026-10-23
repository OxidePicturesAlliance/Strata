//! Values really cross mineral boundaries: each test hands a value produced
//! by one crate to another with no conversion. Most of the guarantee is that
//! these files *compile*: with two copies of a vocabulary crate the type
//! annotations below would be type errors.

use std::any::TypeId;

use strata::half::f16;
use strata::openusd::sdf::{self, AbstractData};
use strata::{glam, ocio, openusd, ptex, vdb};

/// The `half` type every mineral names is the one Strata re-exports.
#[test]
fn one_f16_type() {
    assert_eq!(TypeId::of::<openusd::gf::f16>(), TypeId::of::<f16>());
    assert_eq!(TypeId::of::<vdb::math::Vec3h>(), TypeId::of::<vdb::math::Vec3<f16>>());
}

/// A `half[]` attribute read by openusd becomes the voxel values of a
/// `vdb::Grid<f16>`, and ocio compares what comes back out.
#[test]
fn half_from_usd_into_vdb_compared_by_ocio() {
    let layer = r#"#usda 1.0
def Volume "Smoke"
{
    half[] density = [0.25, 0.5, 1.0, 2.0]
}
"#;
    let data = openusd::usda::parse(layer).expect("parse usda");
    let path = sdf::Path::new("/Smoke.density").expect("path");
    let value = data.field(&path, "default").expect("density").into_owned();
    // openusd hands back its `gf::f16`, annotated here as Strata's `half::f16`.
    let density: Vec<f16> = value.try_into().expect("half[]");
    assert_eq!(density.len(), 4);

    let mut grid = vdb::Grid::<f16>::new(f16::ZERO);
    for (i, d) in density.iter().enumerate() {
        grid.set_value(&vdb::Coord::new(i as i32, 0, 0), *d);
    }
    assert_eq!(grid.active_voxel_count(), 4);

    for (i, d) in density.iter().enumerate() {
        let back: f16 = grid.get_value(&vdb::Coord::new(i as i32, 0, 0));
        // `halfs_differ` takes ocio's `f16`: the same type again.
        assert!(!ocio::ops::lut1d::halfs_differ(*d, back, 0), "voxel {i}");
    }
}

/// ptex encodes halves itself (as `u16`); its codec must agree bit for bit
/// with the platform's `half`, or data moved between ptex and the other
/// minerals would drift.
#[test]
fn ptex_half_codec_agrees_with_half() {
    for x in [0.0_f32, 0.25, 0.5, 1.0, -2.0, 65504.0, 6.1e-5, 1.0e-7] {
        assert_eq!(ptex::float_to_half(x), f16::from_f32(x).to_bits(), "{x}");
    }
    for bits in [0x0000_u16, 0x3c00, 0xc000, 0x7bff, 0x0400, 0x0001, 0x7c00] {
        assert_eq!(ptex::half_to_float(bits), f16::from_bits(bits).to_f32(), "{bits:#06x}");
    }
}

/// Crust re-exports glam; its types are Strata's glam types.
#[test]
fn crust_speaks_strata_glam() {
    let v: glam::Vec3A = crust_core::Vec3A::new(1.0, 2.0, 3.0);
    let m: crust_core::Mat4 = glam::Mat4::from_translation(glam::Vec3::X);
    assert_eq!(m.transform_point3a(v), glam::Vec3A::new(2.0, 2.0, 3.0));
}

/// vdb's tools and Crust run on rayon's global pool; with one rayon there is
/// one pool, and a Strata-built pool drives vdb's parallel code.
#[test]
fn vdb_runs_on_a_strata_rayon_pool() {
    let pool = strata::rayon::ThreadPoolBuilder::new()
        .num_threads(2)
        .build()
        .expect("pool");
    let voxels = pool.install(|| {
        let sphere: vdb::FloatGrid =
            vdb::tools::level_set_sphere::create_level_set_sphere(1.0, vdb::Vec3f::zero(), 0.1, 3.0, true)
                .expect("sphere");
        sphere.active_voxel_count()
    });
    assert!(voxels > 0);
}
