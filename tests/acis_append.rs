//! Writing a kernel-built solid into an ACIS document, and reading it back.
//!
//! The test that matters is the round trip: a body the kernel made, written
//! out and lifted again, has to be the same solid. Anything less and a
//! boolean's output cannot be saved.

use acadifc::acis::{append, lift, Unappendable};
use acadifc::kernel::brep::{boolean, make, Surface};
use cadcodec::entities::acis::types::SatDocument;

#[test]
fn a_box_written_out_comes_back_the_same_box() {
    let solid = make::cuboid([0.0, 0.0, 0.0], [10.0, 20.0, 30.0]).unwrap();
    let mut document = SatDocument::new();
    let written = append(&solid, &mut document).expect("a valid solid writes");
    assert!(written.records > 0);

    let (bodies, loss) = lift(&document);
    assert!(loss.is_empty(), "{loss:?}");
    assert_eq!(bodies.len(), 1);
    let back = &bodies[0];
    assert_eq!(back.faces.len(), 6);
    assert_eq!(back.edges.len(), 12);
    assert_eq!(back.vertices.len(), 8);
    assert_eq!(back.euler_characteristic(), 2);
    let flaws = back.validate();
    assert!(flaws.is_empty(), "{flaws:?}");
}

#[test]
fn it_survives_the_text_as_well_as_the_document() {
    let solid = make::cuboid([1.0, 2.0, 3.0], [4.0, 5.0, 6.0]).unwrap();
    let mut document = SatDocument::new();
    append(&solid, &mut document).unwrap();
    let text = document.to_sat_string();
    let reparsed = SatDocument::parse(&text).expect("what was written parses");
    let (bodies, loss) = lift(&reparsed);
    assert!(loss.is_empty(), "{loss:?}");
    assert_eq!(bodies[0].faces.len(), 6);
    assert_eq!(bodies[0].euler_characteristic(), 2);
}

#[test]
fn the_corners_come_back_where_they_were() {
    let solid = make::cuboid([1.0, 2.0, 3.0], [4.0, 5.0, 6.0]).unwrap();
    let mut document = SatDocument::new();
    append(&solid, &mut document).unwrap();
    let (bodies, _) = lift(&document);
    let mut corners: Vec<[f64; 3]> =
        bodies[0].vertices.iter().map(|(_, v)| v.point).collect();
    corners.sort_by(|a, b| a.partial_cmp(b).unwrap());
    assert_eq!(corners.first(), Some(&[1.0, 2.0, 3.0]));
    assert_eq!(corners.last(), Some(&[5.0, 7.0, 9.0]));
}

#[test]
fn every_face_keeps_the_plane_it_was_on() {
    let solid = make::cuboid([0.0; 3], [2.0, 2.0, 2.0]).unwrap();
    let mut document = SatDocument::new();
    append(&solid, &mut document).unwrap();
    let (bodies, _) = lift(&document);
    let back = &bodies[0];
    // Every corner of every face still lies on that face's own surface,
    // which is what a wrong token position would break.
    for face in back.face_keys() {
        let surface = back
            .surfaces
            .get(back.faces.get(face).unwrap().surface)
            .unwrap();
        assert!(matches!(surface, Surface::Plane(_)));
        for coedge in back.face_coedges(face) {
            let (start, _) = back.coedge_vertices(coedge).unwrap();
            let point = back.vertices.get(start).unwrap().point;
            assert!(surface.contains(point, 1e-9), "{point:?} left its face");
        }
    }
}

#[test]
fn a_boolean_result_can_be_saved() {
    // The reason this exists. A solid the kernel produced has no records
    // behind it, so nothing to update — only records to create.
    let a = make::cuboid([0.0; 3], [10.0; 3]).unwrap();
    let b = make::cuboid([5.0; 3], [10.0; 3]).unwrap();
    let joined = boolean::combine(a, b, boolean::Operation::Union, 1e-9).expect("a union");
    let mut document = SatDocument::new();
    append(&joined, &mut document).expect("the union writes");

    let (bodies, loss) = lift(&document);
    assert!(loss.is_empty(), "{loss:?}");
    let back = &bodies[0];
    assert_eq!(back.faces.len(), joined.faces.len());
    assert_eq!(back.edges.len(), joined.edges.len());
    assert!(back.validate().is_empty());
}

#[test]
fn a_second_body_lands_beside_the_first() {
    let mut document = SatDocument::new();
    append(&make::cuboid([0.0; 3], [1.0; 3]).unwrap(), &mut document).unwrap();
    append(&make::cuboid([9.0; 3], [1.0; 3]).unwrap(), &mut document).unwrap();
    let (bodies, loss) = lift(&document);
    assert!(loss.is_empty(), "{loss:?}");
    assert_eq!(bodies.len(), 2, "the first was not overwritten");
    for body in &bodies {
        assert_eq!(body.faces.len(), 6);
    }
}

#[test]
fn an_inconsistent_body_is_refused_rather_than_written() {
    // Writing a body whose topology does not hold together produces a
    // document that parses into something else, which is worse than not
    // writing it.
    let mut solid = make::cuboid([0.0; 3], [1.0; 3]).unwrap();
    let face = solid.face_keys().next().unwrap();
    solid.faces.remove(face);
    let mut document = SatDocument::new();
    assert_eq!(
        append(&solid, &mut document).err(),
        Some(Unappendable::Inconsistent)
    );
    assert_eq!(document.record_count(), 0, "nothing was added");
}

#[test]
fn a_body_at_survey_coordinates_round_trips() {
    let origin = [512_345.678, 4_512_345.678, 91.5];
    let solid = make::cuboid(origin, [0.5, 0.5, 0.5]).unwrap();
    let mut document = SatDocument::new();
    append(&solid, &mut document).unwrap();
    let text = document.to_sat_string();
    let (bodies, _) = lift(&SatDocument::parse(&text).unwrap());
    let back = &bodies[0];
    assert_eq!(back.euler_characteristic(), 2);
    assert!(back.worst_vertex_gap() < 1e-6, "{}", back.worst_vertex_gap());
    let far = back
        .vertices
        .iter()
        .any(|(_, v)| (v.point[0] - origin[0]).abs() < 1e-6);
    assert!(far, "the coordinates did not survive the text");
}
