// Realistic Jenkinsfile shape — `pipeline` block from
// `SPECIFICATION.md` §4 / §10 row #37 wrapping ordinary closure
// DSL invocations. Zero ERROR / MISSING is the integration-test
// contract.

pipeline {
    agent any

    environment {
        FOO = "bar"
        DEPLOY_TARGET = "staging"
    }

    stages {
        stage('Build') {
            steps {
                sh 'make build'
                archiveArtifacts artifacts: 'build/**/*'
            }
        }

        stage('Test') {
            steps {
                sh 'make test'
            }
            post {
                always {
                    junit 'build/test-results/**/*.xml'
                }
            }
        }

        stage('Deploy') {
            when {
                branch 'main'
            }
            steps {
                sh "deploy.sh ${DEPLOY_TARGET}"
            }
        }
    }

    post {
        always {
            cleanWs()
        }
        failure {
            mail(to: 'team@example.com', subject: "Build failed", body: "see logs")
        }
    }
}
