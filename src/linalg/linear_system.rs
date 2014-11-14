#![doc="Methods for solving linear systems of equations
"]


// std imports

// srmat imports
//use error::*;
use matrix::{MatrixF64, MatrixType, Matrix, 
    MinMaxAbs, 
    ERO, Search};
use error::*;
use linalg::singularity::*;


/// A Gauss elimination problem specification
pub struct GaussElimination<'a, 'b>{
    /// The matrix A of AX = B
    pub a : &'a MatrixF64,
    /// The matrix B of AX = B
    pub b : &'b MatrixF64
}

#[doc="Implements the Gauss elimination algorithm
for solving the linear system AX = B.
"]
impl<'a, 'b> GaussElimination<'a, 'b>{

    /// Setup of a new Gauss elimination problem.
    pub fn new(a : &'a MatrixF64, b : &'b MatrixF64) -> GaussElimination<'a, 'b>{
        assert!(a.is_square());
        assert_eq!(a.num_rows(), b.num_rows());
        GaussElimination{a : a , b : b}
    } 

    /// Carries out the procedure of Gauss elimination.
    pub fn solve(&self) -> Result<MatrixF64,SRError> {
        let mut m = self.a.clone();
        m.append_columns(self.b);
        let rows = m.num_rows();
        let cols = m.num_cols();
        // a vector to hold the positions
        //let mut v  = from_range_uint(rows, 1, 0, rows);
        // Forward elimination process.
        for k in range(0, rows){
            // We are working on k-th column.
            let rr = {
                // Create a view of the remaining elements in column
                let col_k_remaining = m.view(k, k, rows - k, 1);
                //println!("k={}, col_k_remaining: {}", k, col_k_remaining);
                // find the maximum value in this view
                let (_, rr, _) = col_k_remaining.max_abs_scalar();
                // translate rr to the overall row number
                rr + k
            };
            if rr > k {
                // We need to exchange rows of the submatrix.
                m.ero_switch(k, rr);
                // Lets keep this position change information in record
                //v.ero_switch(k, rr);
            }
            // Pick up the pivot
            let pivot = m.get(k, k);
            let mut lower_right  = m.view(k + 1, k, rows - k - 1, cols -k);
            //println!("Pivot: {}", pivot);
            //println!("lower_right: {}", lower_right);
            for r in range(0, lower_right.num_rows()){
                let first = lower_right.get(r, 0);
                let factor = first  / pivot;
                lower_right.ero_scale_add(r, -1, -factor);
            }
            //println!("m: {}", m);
        }
        // Backward substitution starts now.
        let mut b = m.view(0, self.a.num_cols(), 
            self.b.num_rows(), 
            self.b.num_cols());
        let mut r = m.num_rows() - 1;
        loop {
            let pivot = m.get(r, r);
            if pivot == 0. {
                // We have a problem here. We cannot find a solution.
                // TODO: make it more robust for under-determined systems.
                return Err(NoSolution);
            }
            b.ero_scale(r, 1.0/pivot);
            for j in range(r+1, m.num_rows()){
                let factor = m.get(r, j) / pivot;
                b.ero_scale_add(r, j as int, -factor);  
            }
            if r == 0 {
                break;
            }
            r -= 1;
        }
        //println!("m: {}", m);
        // We extract the result.
        Ok(b.to_matrix())
    }
    
}


#[doc="Implements the forward substitution algorithm for
solving a lower triangular linear system. L X = B
"]
pub fn lt_solve(l : &MatrixF64, b : &MatrixF64) -> 
    Result<MatrixF64, SRError>{
    assert!(l.is_square());
    let n = l.num_rows();
    assert_eq!(n, b.num_rows());
    debug_assert!(l.is_lt());
    // Create a copy for the result
    let mut b = b.clone();
    for r in range(0, n) {
        let pivot = l.get(r, r);
        if pivot == 0. {
            // We have a problem here. We cannot find a solution.
            // TODO: make it more robust for under-determined systems.
            return Err(NoSolution);
        }
        for k in range(0,  r){
            b.ero_scale_add(r, k as int, -l.get(r, k));
        }
        b.ero_scale(r, 1.0/pivot);
    }
    Ok(b)
}



#[doc="Implements the back substitution algorithm for
solving a upper triangular linear system. L X = B
"]
pub fn ut_solve(u : &MatrixF64, b : &MatrixF64) -> 
    Result<MatrixF64, SRError>{
    assert_eq!(u.num_rows(), b.num_rows());
    assert!(u.is_square());
    debug_assert!(u.is_ut());
    // Create a copy for the result
    let mut b = b.clone();
    let mut r = u.num_rows() - 1;
    loop {
        let pivot = u.get(r, r);
        if pivot == 0. {
            // We have a problem here. We cannot find a solution.
            // TODO: make it more robust for under-determined systems.
            return Err(NoSolution);
        }
        b.ero_scale(r, 1.0/pivot);
        for j in range(r+1, u.num_rows()){
            let factor = u.get(r, j) / pivot;
            b.ero_scale_add(r, j as int, -factor);  
        }
        if r == 0 {
            break;
        }
        r -= 1;
    }
    Ok(b)
}


#[doc="Implements the algorithm for solving the equation LDU X = B
where L, D, U are known (LDU decomposition of A), B is known and X is unknown.
Uses a combination of forward and backward substitutions. 
"]
pub fn ldu_solve(l : &MatrixF64, 
    d : &MatrixF64,
    u : &MatrixF64,
    b : &MatrixF64) -> 
    Result<MatrixF64, SRError>{
    assert_eq!(b.num_rows(), u.num_rows());
    assert_eq!(l.num_rows(), u.num_rows());
    assert_eq!(d.num_rows(), u.num_rows());
    assert!(u.is_square());
    assert!(l.is_square());
    debug_assert!(!is_singular_lt(l));
    debug_assert!(!is_singular_diagonal(d));
    debug_assert!(!is_singular_ut(u));

    let n = l.num_rows();
    // Create a copy for the result
    let mut b = b.clone();

    // Solve forward substitution problem L X = B
    for r in range(0, n) {
        let pivot = l.get(r, r);
        if pivot == 0. {
            // We have a problem here. We cannot find a solution.
            // TODO: make it more robust for under-determined systems.
            return Err(NoSolution);
        }
        for k in range(0,  r){
            b.ero_scale_add(r, k as int, -l.get(r, k));
        }
        b.ero_scale(r, 1.0/pivot);
    }

    // Perform inverse scaling D X = B
    for r in range(0, n){
        let factor = d.get(r, r);
        b.ero_scale(r, 1.0/factor);
    }

    // Solve backward substitution problem U X = B
    let mut r = u.num_rows() - 1;
    loop {
        let pivot = u.get(r, r);
        if pivot == 0. {
            // We have a problem here. We cannot find a solution.
            // TODO: make it more robust for under-determined systems.
            return Err(NoSolution);
        }
        b.ero_scale(r, 1.0/pivot);
        for j in range(r+1, u.num_rows()){
            let factor = u.get(r, j) / pivot;
            b.ero_scale_add(r, j as int, -factor);  
        }
        if r == 0 {
            break;
        }
        r -= 1;
    }
    Ok(b)
}


/// Computes the inverse of a matrix
pub fn inverse_ero(a : &mut MatrixF64) ->  Result<MatrixF64, SRError>{
    if !a.is_square(){
        return Err(NonSquareMatrix);
    }
    let n = a.num_rows();
    let mut result  : MatrixF64 = Matrix::identity(n, n);
    // forward elimination
    for k in range(0, n){
        let (_, rr) = a.max_abs_scalar_in_col(k, k, n);
        if rr > k {
            // TODO : we can switch only part of row
            a.ero_switch(k, rr);
            result.ero_switch(k, rr);
        }
        let mut v = a.view(k, k, n - k, n - k);
        // Pick the pivot
        let pivot  = v.get(0, 0);
        if pivot == 0. {
            return Err(IsSingular);
        }
        // bring 1 in the diagonal 
        v.ero_scale(0, 1./pivot);
        result.ero_scale(k, 1./pivot);
        for r in range(1, v.num_rows()){
            let first = v.get(r, 0);
            v.ero_scale_add(r, 0, -first);
            // TODO: ignore 0 entries in k-th row of result
            result.ero_scale_add(k + r, k as int, -first);
        }
        //println!("a: {}", a);
        //println!("b: {}", result);
    }
    //println!("a: {}", a);
    //println!("b: {}", result);
    // back substitution
    let mut k = n -1;
    loop {
        // We are using (k, k) entry in a and
        // updating k-th column in a.
        for r in range(0, k){
            let factor = a.get(r, k);
            result.ero_scale_add(r, k as int, -factor);
        }
        if k == 0 {
            break;
        }
        k -= 1;
    }
    //println!("a: {}", a);
    //println!("result: {}", result);
    Ok(result)
}



/// Validates the solution of linear system
pub struct LinearSystemValidator<'a, 'b, 'c>{
    /// The matrix A of AX = B
    pub a : &'a MatrixF64,
    /// The matrix X of AX = B
    pub x : &'b MatrixF64,
    /// The matrix B of AX = B
    pub b : &'c MatrixF64,
    /// The difference matrix 
    pub d : MatrixF64
}

impl<'a, 'b, 'c> LinearSystemValidator<'a, 'b, 'c>{

    /// Setup of a new Gauss elimination problem.
    pub fn new(a : &'a MatrixF64, 
        x : &'b MatrixF64,
        b : &'c MatrixF64,
        ) -> LinearSystemValidator<'a, 'b, 'c>{
        assert_eq!(a.num_rows(), b.num_rows());
        LinearSystemValidator{a : a , 
            x : x, 
            b : b, 
            d : *a * *x - *b}
    }

    pub fn max_abs_scalar_value(&self)-> f64{
        self.d.max_abs_scalar_value()
    }

    /// Validates the equality Ax = b subject to maximum
    /// absolute error being less than a specified threshold.
    pub fn is_max_abs_val_below_threshold(&self, threshold: f64)-> bool{
        self.max_abs_scalar_value() < threshold
    }

    /// Printing for debugging
    pub fn print(&self){
        println!("a: {}", self.a);
        println!("x: {}", self.x);
        println!("ax: {}", *self.a * *(self.x));
        println!("b: {}", self.b);
        println!("diff: {}", self.d);
        println!("max abs diff: {}", self.max_abs_scalar_value());
    }
}





#[cfg(test)]
mod test{
    use super::*;
    use matrix::*;
    use std::num;


    #[test]
    fn test_ge_0(){
        let a = matrix_cw_f64(2,2, [1., 4., 2., 5.]);
        println!("{}", a);
        let b = vector_f64([3.0, 6.0]);
        let x = GaussElimination::new(&a, &b).solve().unwrap();
        println!("{}", x);
        assert_eq!(x, vector_f64([-1., 2.]));
        let lsv = LinearSystemValidator::new(&a, &x, &b);
        assert!(lsv.is_max_abs_val_below_threshold(1e-6));
    }

    #[test]
    fn test_ge_1(){
        let mut a = from_range_cw(3, 3, 1.0, 100.0);
        let x = from_range_cw(3, 1, 1.0, 100.0);
        // a above is rank-2.
        a.set(2,2, 11.0);
        let b  = a * x;
        println!("A: {}", a);
        println!("x: {}", x);
        println!("b: {}", b);
        let ge = GaussElimination::new(&a, &b);
        let z = ge.solve().unwrap();
        println!("z: {}", z);
        /*
        TODO: have better understanding of
        the roundoff error.
        In this case, it is greater than
        1e-15.
        */
        let lsv = LinearSystemValidator::new(&a, &x, &b);
        assert!(lsv.is_max_abs_val_below_threshold(1e-6));
    }  

    #[test]
    fn test_ge_2(){
        let a = matrix_cw_f64(3,3, [1., 0., 0., 
            -1., 1., 0., 
            1., 1., 1.]);
        let b = vector_f64([1., 1., 1.]);
        let ge = GaussElimination::new(&a, &b);
        let x  = ge.solve().unwrap();
        println!("a: {}, x: {}, b: {}", a, x, b);
        // answer is [0, 0, 1]
        let lsv = LinearSystemValidator::new(&a, &x, &b);
        assert!(lsv.is_max_abs_val_below_threshold(1e-6));
    }

    #[test]
    fn test_ge_3(){
        let a = matrix_cw_f64(3,3, [2., 4., -2., 
            1., -6., 7., 
            1., 0., 2.]);
        let b = vector_f64([5., -2., 9.]);
        let ge = GaussElimination::new(&a, &b);
        let x  = ge.solve().unwrap();
        println!("a: {}, x: {}, b: {}", a, x, b);
        // answer is [1, 1, 2]
        //assert!(false);
        let lsv = LinearSystemValidator::new(&a, &x, &b);
        assert!(lsv.is_max_abs_val_below_threshold(1e-6));
    }

    #[test]
    fn test_ge_no_solution(){
        let a = matrix_cw_f64(2,2, [1., 4., 2., 8.]);
        println!("{}", a);
        let b = vector_f64([3.0, 6.0]);
        let result = GaussElimination::new(&a, &b).solve();
        match result {
            Ok(x) => {
                println!("{}", x);
                assert!(false);
            },
            Err(e) => println!("{}", e),
        }
        
    }

    #[test]
    fn test_ut_0(){
        let l = matrix_rw_f64(2, 2, [
            1., 1.,
            0., 1.]);
        let x = vector_f64([1., 2.]);
        let b = l * x;
        let x = ut_solve(&l, &b).unwrap();
        let lsv = LinearSystemValidator::new(&l, &x, &b);
        lsv.print();
        assert!(lsv.is_max_abs_val_below_threshold(1e-6));
    }

    #[test]
    fn test_lt_0(){
        let l = matrix_rw_f64(2, 2, [
            1., 0.,
            1., 1.]);
        let x = vector_f64([1., 2.]);
        let b = l * x;
        let x = lt_solve(&l, &b).unwrap();
        let lsv = LinearSystemValidator::new(&l, &x, &b);
        lsv.print();
        assert!(lsv.is_max_abs_val_below_threshold(1e-6));
    }



    #[test]
    fn test_inv_ero_0(){
        let a = matrix_rw_f64(2, 2, [
            1., 0.,
            1., 1.]);
        let b = inverse_ero(&mut a.clone()).unwrap();
        let c = a * b;
        assert!(c.is_identity());

    }

    #[test]
    fn test_inv_ero_hadamard(){
        for i in range(2, 6){
            let n = num::pow(2,i);
            let a = hadamard(n).unwrap();
            println!("n: {}", n);
            println!("a: {}", a);
            let b = inverse_ero(&mut a.clone()).unwrap();
            let c = a * b;
            println!("b: {}", b);
            println!("c: {}", c);

            assert!(c.is_identity());
        }

    }

    #[test]
    fn test_inv_ero_hilbert(){
        for n in range(2, 10){
            let a = hilbert(n);
            println!("n: {}", n);
            println!("a: {}", a);
            let b = inverse_ero(&mut a.clone()).unwrap();
            let c = a * b;
            println!("b: {}", b);
            println!("c: {}", c);
            let i : MatrixF64  = Matrix::identity(n, n);
            /*
            Hilbert matrices are badly conditioned. Hence,
            the numerical accuracy fails. We don't really
            get a proper identity matrix. What we get
            is a matrix which is close to identity matrix.

            A suitable way to verify correctness is to 
            measure the maximum deviation of any entry
            int the difference matrix below from identity. 
            */
            let diff = i  - c;
            let max = diff.max_abs_scalar_value();
            assert!(max < 1e-3);
        }

    }

}
