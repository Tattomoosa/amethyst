use crate::{
    camera::{ActiveCamera, Camera},
    hidden::{Hidden, HiddenPropagate},
    transparent::Transparent,
};
use amethyst_core::{
    ecs::prelude::{
        Component, DenseVecStorage, Entities, Entity, Join, Read, ReadStorage, System, Write,
    },
    math::{distance_squared, Matrix4, Point3, Vector4},
    GlobalTransform,
};
use hibitset::BitSet;
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;

#[cfg(feature = "profiler")]
use thread_profiler::profile_scope;

/// Resource for controlling what entities should be rendered, and whether to draw them ordered or
/// not, which is useful for transparent surfaces.
#[derive(Default)]
pub struct Visibility {
    /// Visible entities that can be drawn in any order
    pub visible_unordered: BitSet,
    /// Visible entities that need to be drawn in the given order
    pub visible_ordered: Vec<Entity>,
}

/// Determine what entities are visible to the camera, and which are not. Will also sort transparent
/// entities back to front based on distance from camera.
///
/// Note that this should run after `GlobalTransform` has been updated for the current frame, and
/// before rendering occurs.
pub struct VisibilitySortingSystem {
    centroids: Vec<Internals>,
    transparent: Vec<Internals>,
}

/// Defines a object's bounding sphere used by frustum culling.
#[derive(Clone, PartialEq, Serialize, Deserialize)]
pub struct BoundingSphere {
    pub center: Point3<f32>,
    pub radius: f32,
}

impl Default for BoundingSphere {
    fn default() -> Self {
        Self {
            center: Point3::origin(),
            radius: 1.0,
        }
    }
}

impl BoundingSphere {
    pub fn new(center: Point3<f32>, radius: f32) -> Self {
        Self { center, radius }
    }

    pub fn origin(radius: f32) -> Self {
        Self {
            center: Point3::origin(),
            radius,
        }
    }
}

impl Component for BoundingSphere {
    type Storage = DenseVecStorage<Self>;
}

#[derive(Clone)]
struct Internals {
    entity: Entity,
    transparent: bool,
    centroid: Point3<f32>,
    camera_distance: f32,
}

impl VisibilitySortingSystem {
    /// Create new sorting system
    pub fn new() -> Self {
        VisibilitySortingSystem {
            centroids: Vec::default(),
            transparent: Vec::default(),
        }
    }
}

impl<'a> System<'a> for VisibilitySortingSystem {
    type SystemData = (
        Entities<'a>,
        Write<'a, Visibility>,
        ReadStorage<'a, Hidden>,
        ReadStorage<'a, HiddenPropagate>,
        Option<Read<'a, ActiveCamera>>,
        ReadStorage<'a, Camera>,
        ReadStorage<'a, Transparent>,
        ReadStorage<'a, GlobalTransform>,
        ReadStorage<'a, BoundingSphere>,
    );

    fn run(
        &mut self,
        (entities, mut visibility, hidden, hidden_prop, active, camera, transparent, global, bound): Self::SystemData,
    ) {
        #[cfg(feature = "profiler")]
        profile_scope!("run");

        let origin = Point3::origin();
        let defcam = Camera::standard_2d();
        let identity = GlobalTransform::default();

        let mut camera_join = (&camera, &global).join();
        let (camera, camera_transform) = active
            .and_then(|a| camera_join.get(a.entity, &entities))
            .or_else(|| camera_join.next())
            .unwrap_or((&defcam, &identity));

        let camera_centroid = camera_transform.0.transform_point(&origin);
        let frustum = Frustum::new(camera.proj * camera_transform.0.try_inverse().unwrap());

        self.centroids.clear();
        self.centroids.extend(
            (&*entities, &global, bound.maybe(), !&hidden, !&hidden_prop)
                .join()
                .map(|(entity, global, sphere, _, _)| {
                    let pos = sphere.map_or(&origin, |s| &s.center);
                    (
                        entity,
                        global.0.transform_point(&pos),
                        sphere.map_or(1.0, |s| s.radius)
                            * global.0[(0, 0)].max(global.0[(1, 1)]).max(global.0[(2, 2)]),
                    )
                })
                .filter(|(_, centroid, radius)| frustum.check_sphere(centroid, *radius))
                .map(|(entity, centroid, _)| Internals {
                    entity,
                    transparent: transparent.contains(entity),
                    centroid,
                    camera_distance: distance_squared(&centroid, &camera_centroid),
                }),
        );
        self.transparent.clear();
        self.transparent
            .extend(self.centroids.iter().filter(|c| c.transparent).cloned());

        self.transparent.sort_by(|a, b| {
            b.camera_distance
                .partial_cmp(&a.camera_distance)
                .unwrap_or(Ordering::Equal)
        });

        visibility.visible_unordered.clear();
        visibility.visible_unordered.extend(
            self.centroids
                .iter()
                .filter(|c| !c.transparent)
                .map(|c| c.entity.id()),
        );

        visibility.visible_ordered.clear();
        visibility
            .visible_ordered
            .extend(self.transparent.iter().map(|c| c.entity));
    }
}

#[derive(Debug)]
struct Frustum {
    planes: [Vector4<f32>; 6],
}

impl Frustum {
    fn new(matrix: Matrix4<f32>) -> Self {
        let planes = [
            (matrix.row(3) + matrix.row(0)).transpose(),
            (matrix.row(3) - matrix.row(0)).transpose(),
            (matrix.row(3) - matrix.row(1)).transpose(),
            (matrix.row(3) + matrix.row(1)).transpose(),
            (matrix.row(3) + matrix.row(2)).transpose(),
            (matrix.row(3) - matrix.row(2)).transpose(),
        ];
        Self {
            planes: [
                planes[0] * (1.0 / planes[0].xyz().magnitude()),
                planes[1] * (1.0 / planes[1].xyz().magnitude()),
                planes[2] * (1.0 / planes[2].xyz().magnitude()),
                planes[3] * (1.0 / planes[3].xyz().magnitude()),
                planes[4] * (1.0 / planes[4].xyz().magnitude()),
                planes[5] * (1.0 / planes[5].xyz().magnitude()),
            ],
        }
    }

    fn check_sphere(&self, center: &Point3<f32>, radius: f32) -> bool {
        for plane in &self.planes {
            if plane.xyz().dot(&center.coords) + plane.w <= -radius {
                return false;
            }
        }
        return true;
    }
}
